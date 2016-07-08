Avery operating system
================
[![Build Status](https://travis-ci.org/AveryOS/avery.svg?branch=master)](https://travis-ci.org/AveryOS/avery)

Avery is an operating system written in Rust designed around fast remote procedure calls and capability-based security. To achieve fast remote procedure calls on x86 it uses software isolated processes in a single address space. The isolation is done using an LLVM IR pass which transforms the IR into a form where it is trivial to prove isolation. This means that you currently need a LLVM based compiler (like clang or rustc) to compile code for this OS. The kernel has a unrelated verifier which ensures that any loaded code must be isolating, which means that the compiler stack does not need to be trusted.

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
  * cmake 3.6
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
    * `brew install git openssl cmake ninja qemu autoconf`

# Building

You can then build the kernel with `rake`. This will take a while as it builds LLVM, Clang, Rust and other things. `rake qemu` builds and runs the kernel in QEMU.
