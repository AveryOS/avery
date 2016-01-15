This requires git, curl, libiconv, bison, patch, diffutils, texinfo, libssl-dev (on Unix), gcc, cmake, ninja (on Windows) and rake to be installed. OS X comes with all dependencies (with command line tools installed). On Windows, all commands must be run in a [MSYS2](https://msys2.github.io/) MINGW shell. The dependencies can be installed in MSYS2 by running `rake deps_msys`.

Run `rake deps` to set up and build dependencies. This will take a while as it builds LLVM, rustc and other things.

You can then build the kernel with `rake`. `rake qemu` runs it in QEMU.
