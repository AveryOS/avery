# Dependencies
  * git
  * curl
  * libiconv
  * bison
  * patch
  * diffutils
  * texinfo
  * libssl-dev
  * gcc
  * cmake
  * rake
  * ninja (optional)
  * qemu (optional)

## Installing dependencies on Windows
  * Install and update [MSYS2](https://msys2.github.io/)
  * All commands must be run in a MSYS2 MINGW shell
  * Run `rake deps_msys`

## Installing dependencies on OS X  
  * Command line tools, `xcode-select --install`
  * Using [Homebrew](http://brew.sh/)
    * `brew install git openssl cmake ninja qemu`
    * `brew link openssl --force`

# Building

Run `rake deps` to set up and build OS dependencies. This will take a while as it builds LLVM, rustc and other things.

You can then build the kernel with `rake`. `rake qemu` runs it in QEMU.
