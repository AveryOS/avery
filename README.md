This requires git, curl, libiconv, bison, patch, gcc, rustc and rake to be installed. OS X comes with most of these (with command line tools installed).

Run `rake deps_unix` to set up and build dependencies requiring a POSIX system. On Windows this must run in [MSYS2](https://msys2.github.io/) (and not with the MINGW shells). Be sure to install the dependencies in MSYS2 (including GCC).

Run `rake deps` to set up and build dependencies. On Windows this must also run in [MSYS2](https://msys2.github.io/) (optionally in a MINGW shell).

The above tasks can be run with `rake setup` in a POSIX system (MSYS2 shell included).

You can then build the kernel with `rake`. `rake qemu` runs it in QEMU.
