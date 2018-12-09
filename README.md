# bivrost!
A socket server to shared socket descriptor bridge.

## General
bivrost! is a small utility mostly for working with DOOR32.SYS style socket file descriptor sharing in conjunction with systems such as [ENiGMA½ BBS](https://github.com/NuSkooler/enigma-bbs/) that do not support the feature directly. With bivrost! you can bridge the two.

bivrost! is released under [Phenom Productions](https://www.phenomprod.com/) BBS mods! Snag a fresh release from your favorite super snazzy-pants BBS today!

## Usage
bivrost! is fairly simple. Below is the current `--help` output:
```
bivrost! A socket server to shared socket descriptor bridge.
Copyright (c) 2018, Bryan D. Ashby

Usage: bivrost --port=<port> [--dropfile=<dropfile> --out=<out>] <target>
       bivrost --help | --version

Options:
  -h, --help             Show this message.
  --version              Show the version of bivrost.
  --port=<port>          Set server port in which to connect to.
  --dropfile=<dropfile>  Set DOOR32.SYS dropfile path.
  --out=<out>            Set output directory for new DOOR32.SYS.
                         If not specified, original DOOR32.SYS will
                         be overridden.

Notes:
  If <target> contains arguments, it should be quoted. For example:
  "DOOR.EXE /D -N 1".

  Arguments within <target> may also contain {fd} which will be
  substituted with the shared socket descriptor (the same value to be found
  in the output DOOR32.SYS).

  If your door does not use DOOR32.SYS you can omit --dropfile and --out and
  still use the {fd} variable.
```

Standard usage currently mostly falls into one of two forms:

1. Reading and producing a new `DOOR32.SYS` for a door to consume.
2. Direct use where a `DOOR32.SYS` is not involved at all. For example, with [NetFoss](http://pcmicro.com/netfoss/) and 16-bit doors such as LORD. In this pattern, the `{fd}` param will be of use to you.

### Example
The following illustrates setting up [Jezebel](http://www.dreamlandbbs.org/jezebel/) under Windows:

First, your `menu.hjson` may have an entry similar to the following:
```hjson
doorJezebel: {
    desc: Jezebel
    module: abracadabra
    config: {
        name: Jezebel
        dropFileType: DOOR32
        cmd: "C:\\enigma-bbs\\utils\\bivrost.exe"
        args: [
            "--port"
            "{srvPort}",            //  bivrost! will connect this port on localhost
            "--dropfile",
            "{dropFilePath}",       //  ...and read this DOOR32.SYS produced by ENiGMA½
            "--out",
            "C:\\doors\\jezebel",   //  ...and produce a NEW DOOR32.SYS here.

            //
            //  Note that the final <target> params bivrost! will use to
            //  launch the door are grouped here. The {fd} variable could
            //  also be supplied here if needed.
            //
            "C:\\doors\\jezebel\\jezw32.exe C:\\doors\\jezebel\\door32.sys"
        ],
        nodeMax: 1
        tooManyArt: DOORMANY
        io: socket
    }
}
```

We've now told ENiGMA½ to launch Jezebel by way of proxy through bivrost!. When launched, bivrost! will perform the following basic steps:

1. Connect up to the ENiGMA½ temporary `socket` server port supplied via `--port`.
2. Read the `DOOR32.SYS` supplied at `--dropfile` and produce a new `DOOR32.SYS` containing the shared socket descriptor that Jezebel wants in the `--out` directory.
3. Finally, launch Jezebel.

