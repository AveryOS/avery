This requires git, curl, libiconv, bison, patch, rustc and rake to be installed. OS X comes with these (with command line tools installed).

Run `rake deps_unix` and `rake deps` to set up and build dependencies. On Windows both of these must run in [MSYS2](https://msys2.github.io/). Be sure to install the dependencies in MSYS2. `rake deps_unix` must be run in the MSYS2 shell and not the MINGW shells.

You then should run `rake match_rustc` to ensure the rustc sources matches the rustc version installed. Run this again if you update rustc.

You can then build the kernel with `rake`. `rake qemu` runs it in QEMU.
