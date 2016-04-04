Avery kernel
================
[![Build Status](https://travis-ci.org/AveryOS/avery.svg?branch=master)](https://travis-ci.org/AveryOS/avery)

# Dependencies
  * git
  * curl
  * libiconv
  * bison
  * patch
  * diffutils
  * texinfo
  * libssl-dev
  * libtool
  * autoconf
  * automake
  * python
  * gcc
  * cmake
  * rake
  * ninja (optional)
  * qemu (optional)

## Installing dependencies on Windows
  * Install and update [MSYS2](https://msys2.github.io/)
  * All commands must be run in a MSYS2 MINGW shell
  * Run `pacman -S ruby` and then run `rake deps_msys`

## Installing dependencies on OS X  
  * Install command line tools, `xcode-select --install`
  * Using [Homebrew](http://brew.sh/)
    * `brew install git openssl cmake ninja qemu`

# Building

You can then build the kernel with `rake`. This will take a while as it builds LLVM, Clang, Rust and other things. `rake qemu` builds and runs the kernel in QEMU.
