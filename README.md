This requires git, curl, libiconv, bison, patch, rustc and rake to be installed. OS X comes with these (with command line tools installed).

Run `rake vendor` to set up and build dependencies. This doesn't work on Windows and it expects to find mtools and binutils already built in `vendor/install/bin/`.

You then should run `rake update` to ensure the rustc sources matches the rustc version installed. Run this again if you update rustc.

You can then build the bootstrap dependencies with `rake base` and finally the kernel with `rake`.