This requires wget, libiconv, bison, rake, rustc and rake to be installed.

On Windows it expects to find mtools and binutils in `bin/`. binutils can be built from source on other platforms with `vendor/binutils/build.sh` provided with the source for binutils in `vendor/binutils/src`.
The source of rustc must be placed in `vendor/rustc`.

You can then build the dependencies with `rake base` and finally the kernel with `rake`.