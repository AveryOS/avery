DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

export PREFIX="$DIR/install"
export TARGET=x86_64-elf
export PATH="$PREFIX/bin:$PATH"
cd "$DIR"
mkdir build
cd build
../src/configure --target=$TARGET --prefix="$PREFIX" --with-sysroot --disable-nls --disable-werror
make -j4
make install