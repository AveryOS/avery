This requires git, curl, libiconv, bison, patch, gcc, rustc and rake to be installed. OS X comes with these (with command line tools installed).

Run `rake deps_unix` to set up and build dependencies requiring a POSIX system. On Windows this must run in [MSYS2](https://msys2.github.io/) (and not with the MINGW shells). Be sure to install the dependencies in MSYS2 (including GCC).

You then should run `rake match_rustc` to ensure the rustc sources matches the rustc version installed. Run this again if you update rustc.

Run `rake deps` to set up and build dependencies. On Windows this must also run in [MSYS2](https://msys2.github.io/) (optionally in a MINGW shell).

You can then build the kernel with `rake`. `rake qemu` runs it in QEMU.
