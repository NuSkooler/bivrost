/*

 ::::: __________________________________________________________________ :::::
 : ____\ .__ .__ _____ __. ____ ___ _______ .__ ______ .__ _____ .__ _. /____ :
 __\ .___! __|_/__    / _|__   /  /_____  __|  \ gRK __|_ \  __  |_ \ !___. /__
 \   ! ___/  |/  /___/  |   \__\ ._/  __\/  \   \___/  |/  \/  \_./  \___ !   /
 /__  (___   /\____\____|\   ____|   /  /___|\   ______.    ____\|\   ___)  __\
   /____  \_/ ___________ \_/ __ |__/ _______ \_/ ____ |___/ _____ \_/  ____\
 :     /________________________________________________________________\     :
 :::::       +  p  H  E  N  O  M  p  R  O  D  U  C  T  I  O  N  S  +      :::::
 ==============================================================================
 bivrost! A socket server to shared socket descriptor bridge.

 Copyright (c) 2018, Bryan D. Ashby
 See LICENSE.TXT

*/
extern crate clap;
extern crate codepage_437;
extern crate docopt;
#[macro_use] extern crate serde_derive;
#[cfg(windows)] extern crate winapi;

use codepage_437::CP437_CONTROL;
use codepage_437::FromCp437;
use codepage_437::IntoCp437;
use clap::crate_version;
use docopt::Docopt;
use std::fs;
use std::io::Error;
use std::net::TcpStream;
use std::os::windows::io::AsRawSocket;
use std::os::windows::raw::HANDLE;
use std::path::Path;
use std::process;
use std::process::Command;
use std::vec::Vec;
use winapi::shared::minwindef::TRUE;
use winapi::um::handleapi::DuplicateHandle;
use winapi::um::processthreadsapi::GetCurrentProcess;
use winapi::um::winnt::DUPLICATE_SAME_ACCESS;

const USAGE: &'static str = "
bivrost! A socket server to shared socket descriptor bridge.
Copyright (c) 2018, Bryan D. Ashby

Usage: bivrost --port=<port> [--dropfile=<dropfile> --out=<out>] <target>
       bivrost --help | --version

Options:
  -h, --help             Show this message.
  --version              Show the version of bivrost!
  --port=<port>          Set server port in which to connect to.
  --dropfile=<dropfile>  Set DOOR32.SYS dropfile path.
  --out=<out>            Set output directory for new DOOR32.SYS.
                         If not specified, original DOOR32.SYS will
                         be overridden.

Notes:
  If <target> contains arguments, it should be quoted. For example: \"DOOR.EXE /D -N 1\"

  Arguments within <target> may also contain {fd} which will be substituted with the
  shared socket descriptor (the same value to be found in the output DOOR32.SYS).

  If your door does not use DOOR32.SYS you can omit --dropfile and --out and still
  use {fd}.";

const DOOR32_SYS_FILENAME: &'static str = "DOOR32.SYS";

#[derive(Debug, Deserialize)]
struct Args {
    flag_port:      i32,
    flag_dropfile:  String,
    flag_out:       String,
    arg_target:     String,   
    flag_version:   bool
}

//
//  DOOR32.SYS
//  https://github.com/NuSkooler/ansi-bbs/blob/master/docs/dropfile_formats/door32_sys.txt
//
fn read_door32sys_dropfile(dropfile_path: &str) -> Result<String, String> {
    let path = Path::new(dropfile_path);
    let is_door32_filename = match path.file_name() {
        Some(ref f) => f.to_string_lossy().eq_ignore_ascii_case(DOOR32_SYS_FILENAME),
        None        => false,
    };

    if !is_door32_filename {        
        return Err(format!("File at {} does not appear to be DOOR32.SYS", dropfile_path));
    }

    match fs::read(path) {
        Ok(contents)    => Ok(String::from_cp437(contents, &CP437_CONTROL)),
        Err(e)          => Err(e.to_string()),
    }
}

#[cfg(windows)]
fn dropfile_filename(filename: &str) -> String {
    filename.to_lowercase()
}

#[cfg(not(windows))]
fn dropfile_filename(filename: &str) -> String {
    filename
}

fn write_new_door32sys_dropfile(
    orig_contents: &str, out_path: &Path, socket_fd: i64) -> Result<String, String>
{
    if !out_path.is_dir() {
        return Err(format!("{} is not a directory", out_path.to_string_lossy()));
    }

    let dropfile_path = out_path.join(dropfile_filename(DOOR32_SYS_FILENAME));

    //
    //  First two lines are as follows:
    //  1 - Comm type (2=telnet)
    //  2 - Shared socket fd
    //  ...the rest is just copied over from the original.
    //      
    let mut contents = format!("2\r\n{}\r\n", socket_fd);
    let remaining_lines = orig_contents.lines().skip(2);
    for line in remaining_lines {
        contents.push_str(&format!("{}\r\n", line));
    }

    match contents.to_string().into_cp437(&CP437_CONTROL) {
        Ok(cp437)   => {
            match fs::write(&dropfile_path, cp437) {
                Ok(()) => {
                    println!("Created new dropfile at {}", dropfile_path.to_string_lossy());
                    Ok(dropfile_path.to_string_lossy().to_string())
                },
                Err(e) => Err(e.to_string()),
            }
        },
        Err(e)      => Err(format!("Failed to convert {} to CP437", e.into_string())),
    }    
}

#[cfg(windows)]
fn get_socket_fd(stream: TcpStream) -> Result<i64, String> {
    let sock_handle = stream.as_raw_socket() as HANDLE;
    let mut dupe_handle: HANDLE = 0 as HANDLE;
    let dupe_handle_ptr: *mut HANDLE = &mut dupe_handle;

    let ret = unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            sock_handle,
            GetCurrentProcess(),
            dupe_handle_ptr,
            0,
            TRUE,
            DUPLICATE_SAME_ACCESS
        )
    };

    if ret == TRUE {
        Ok(dupe_handle as i64)
    } else {
        Err(format!("Failed to duplicate handle: {}", Error::last_os_error().to_string()))
    }
}

#[cfg(not(windows))]
fn get_socket_fd(stream: TcpStream) -> Result<i64, String> {
    Ok(stream.as_raw_fd() as i64)
}

fn connect_to_supplied_port(port: i32) -> Result<TcpStream, String> {
    let address = format!("localhost:{}", port);
    println!("Connecting to {}...", address);

    match TcpStream::connect(address) {
        Ok(stream)  => Ok(stream),
        Err(e)      => Err(e.to_string()),
    }
}

const EXIT_SUCCESS: i32 = 0;
const EXIT_FAILURE: i32 = (-1);

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());

    if args.flag_version {
        println!("{}", crate_version!());
        process::exit(EXIT_SUCCESS);
    }

    let stream = connect_to_supplied_port(args.flag_port)
        .unwrap_or_else(|e| {
            println!("Failed to connect: {}", e.to_string());
            process::exit(EXIT_FAILURE);
        });

    let shared_fd = get_socket_fd(stream)
        .unwrap_or_else(|e| {
            println!("{}", e.to_string());
            process::exit(EXIT_FAILURE);
        });

    println!("Connected. Socket file descriptor is {}", shared_fd);

    if args.flag_dropfile.len() > 0 {
        let dropfile = read_door32sys_dropfile(&args.flag_dropfile)
            .unwrap_or_else(|e| {
                println!("Failed to read dropfile at {}: {}", args.flag_dropfile, e.to_string());
                process::exit(EXIT_FAILURE);
            });

        let out_path = match args.flag_out.is_empty() {
                true => {
                    let p = Path::new(&args.flag_dropfile);
                    p.parent().unwrap()
                },
                false => Path::new(&args.flag_out),
            };            

        write_new_door32sys_dropfile(&dropfile, &out_path, shared_fd)
            .unwrap_or_else(|e| {
                println!("{}", e.to_string());
                process::exit(EXIT_FAILURE);
            });
    }

    let split_args: Vec<String> = args.arg_target.split(' ').map(|a| a.to_string()).collect();
    let mut target_args: Vec<String> = split_args
        .iter()
        .map(|arg| arg.replace("{fd}", &format!("{}", shared_fd)))
        .collect::<Vec<String>>();

    println!("{:?}", target_args);
   
    let command = target_args.first().unwrap();

    let target_exit_status = Command::new(command)
        .args(target_args.split_off(1))
        .status()
        .unwrap_or_else(|e| {
            println!("Execute failed: {}", e.to_string());
            process::exit(EXIT_FAILURE);
        });

    match target_exit_status.code() {
        Some(code)  => {
            println!("Process exited with code {}", code);
            process::exit(code);
        },
        None        => {
            println!("Process terminated by signal");
            process::exit(EXIT_FAILURE);
        }
    }
}
