require 'fileutils'
require_relative 'rake/build'
require_relative 'rake/lokar'

RUSTSHORT = false
AVERY_DIR = File.expand_path('../', __FILE__)
Dir.chdir(AVERY_DIR)

CLEANENV = ENV.to_h

ENV['CARGO_HOME'] = File.expand_path('../build/cargo/home', __FILE__)
ENV['CARGO_TARGET_DIR'] = File.expand_path('../build/cargo/target', __FILE__)
ENV['RUST_TARGET_PATH'] = File.expand_path('../targets', __FILE__)
ENV['RUST_BACKTRACE'] = '1'
UNIX_EMU = [false]

def which(cmd)
	exts = ENV['PATHEXT'] ? ENV['PATHEXT'].split(';') : ['']
	ENV['PATH'].split(File::PATH_SEPARATOR).each do |path|
		exts.each { |ext|
			exe = File.join(path, "#{cmd}#{ext}")
			return exe if File.executable?(exe) && !File.directory?(exe)
		}
	end
	return nil
end

NINJA = which('ninja')

def mkdirs(target)
	FileUtils.makedirs(target)
end

def run(*cmd)
	puts cmd.map { |c| c.shellescape }.join(" ")

	# Run commands in the MSYS shell
	if ON_WINDOWS_MINGW && UNIX_EMU[0]
		msystem = ENV['MSYSTEM']
		ENV['MSYSTEM'] = 'MSYS'
		system('bash', '-lc', "cd #{Dir.pwd.shellescape}; " + cmd.map { |c| c.shellescape }.join(" "))
		ENV['MSYSTEM'] = msystem
	else
		 system([cmd.first, cmd.first], *cmd[1..-1])
	end
	raise "Command #{cmd.join(" ")} failed with error code #{$?}" if $? != 0
end

raise "Install and use MSYS2 Ruby" if ENV['MSYSTEM'] && Gem.win_platform?

ON_WINDOWS = Gem.win_platform? || ENV['MSYSTEM']

ON_WINDOWS_MINGW = ENV['MSYSTEM'] && ENV['MSYSTEM'].start_with?('MINGW')

EXE_POST = ON_WINDOWS ? ".exe" :	""

def append_path(path)
	if Gem.win_platform?
		ENV['PATH'] = "#{path.gsub('/', '\\')};#{ENV['PATH']}"
	else
		ENV['PATH'] = "#{path}:#{ENV['PATH']}"
	end
end

append_path(File.expand_path('../vendor/elf-binutils/install/bin', __FILE__))
append_path(File.expand_path('../vendor/mtools/install/bin', __FILE__))
append_path(File.expand_path('../vendor/llvm/install/bin', __FILE__))
append_path(File.expand_path('../vendor/binutils/install/bin', __FILE__))
append_path(File.expand_path('../vendor/cargo/install/bin', __FILE__))

if RUSTSHORT
	ARCH = `./vendor/config.guess`.strip.sub(/[0-9\.]*$/, '')
	ENV['DYLD_LIBRARY_PATH'] = File.expand_path("../vendor/rust/build/#{ARCH}/stage0/lib/rustlib/#{ARCH}/lib", __FILE__)
	append_path(File.expand_path("../vendor/rust/build/#{ARCH}/stage1/bin", __FILE__))
else
	ENV['DYLD_LIBRARY_PATH'] = File.expand_path("../vendor/rust/install/lib", __FILE__)
	append_path(File.expand_path("../vendor/rust/install/bin", __FILE__))
end

QEMU_PATH = "#{'qemu/' if !which("qemu-system-x86_64")}"
AR = 'x86_64-elf-ar'
AS = 'x86_64-elf-as'
LD = 'x86_64-elf-ld'

def preprocess(input, output, binding)
	content = File.open(input, 'r') { |f| f.read }
	output_content = Lokar.render(content, input, binding)
	File.open(output, 'w') { |f| f.write output_content }
end

def assemble(build, source, objects)
	object_file = source.output(".o")
	build.process object_file, source.path do
		run AS, source.path, '-o', object_file
	end

	objects << object_file
end
#--llvm-args=--inline-threshold=0 # ,

# We need to pass along sysroot here so rustc won't try to use host crates. The sysroot folder doesn't need to exist.
# opt-level=1 is needed so LLVM will optimize out uses of floating point in libcore
RUSTFLAGS = ['-C',"ar=x86_64-elf-ar", '--sysroot', File.expand_path('../build/sysroot', __FILE__)] +
	%w{-C opt-level=1 -C debuginfo=1 -Z no-landing-pads}
ENV['RUSTFLAGS'] = RUSTFLAGS.join(" ") # Cargo uses this. How to pass spaces here?

def build_libcore(build, crate_prefix, flags)
	crates = build.output(File.join(crate_prefix, "crates"))
	mkdirs(crates)
	run 'rustc', *RUSTFLAGS, *flags, 'vendor/rust/src/src/libcore/lib.rs', '--out-dir', crates

	# libcore needs rlibc
	run 'rustc', '-L', crates, *RUSTFLAGS, *flags, '--crate-type', 'rlib', '--crate-name', 'rlibc', 'vendor/rlibc/src/src/lib.rs', '--out-dir', crates
end

def build_crate(build, crate_prefix, out_prefix, flags, src, src_flags)
	crates = build.output(File.join(crate_prefix, "crates"))
	out_prefix = build.output(out_prefix)
	mkdirs(out_prefix)
	run 'rustc', '-C', 'target-feature=-mmx,-sse,-sse2', '-C', 'lto', '-L', crates, '-L', 'build/phase', *RUSTFLAGS, *flags, src,  '--out-dir', out_prefix, *src_flags
end

kernel_object_bootstrap = "build/bootstrap.o"

type = :multiboot
build_kernel = proc do
	build = Build.new('build', 'info.yml')
	kernel_binary = build.output "#{type}/kernel.elf"
	kernel_object = build.output "#{type}/kernel.o"
	kernel_bc = build.output "#{type}/kernel.ll"

	sources = build.package('kernel/**/*')

	efi_files = sources.extract('kernel/arch/x64/efi/**/*')
	multiboot_files = sources.extract('kernel/arch/x64/multiboot/**/*')

	if type == :multiboot
		sources.add multiboot_files
	else
		sources.add efi_files
	end

	bitcodes = []
	bitcodes_bootstrap = []
	objects = ['vendor/font.o', kernel_object]

	build.run do
		# Build the kernel object
		flags = ['--emit=obj,llvm-ir']
		flags += ['--cfg', 'multiboot'] if type == :multiboot
		build_crate(build, "", "#{type}", %w{--target x86_64-avery-kernel}, 'kernel/kernel.rs', flags)

		# Preprocess files

		gen_folder = "gen/#{type}"

		linker_script = "kernel/arch/x64/kernel.ld"
		generated_files = ['kernel/arch/x64/interrupts.s', linker_script]

		generated_files.each do |file|
			sources.extract(file)
			output = build.output File.join(gen_folder, file)
			build.process output, file do |o, i|
				# Linker script requires `multiboot` variable to be defined
				multiboot = type == :multiboot

				# Preprocess the file
				preprocess(file, output, binding)
			end
			sources.add build.package(output)
		end

		# Build all assembly files

		sources.each do |source|
			case source.ext.downcase
				when '.s'
					assemble(build, source, objects)
			end
		end

		puts "Linking..."

		# Add 32-bit multiboot bootstrapper if needed
		objects << kernel_object_bootstrap if type == :multiboot

		# Mark debug sections as loadable
		objects.each do |obj|
			run 'x86_64-elf-objcopy', '--set-section-flags', '.debug*=alloc,contents,load,readonly,data,debug', obj
		end

		# Finally link
		run LD, '-z', 'max-page-size=0x1000', '-T', build.output(File.join(gen_folder, linker_script)), *objects, '-o', kernel_binary

		# Copy kernel into emulation environment
		case type
			when :multiboot
				run 'mcopy', '-D', 'o', '-D', 'O', '-i' ,'emu/grubdisk.img@@1M', kernel_binary, '::kernel.elf'
			when :boot
				FileUtils.cp kernel_binary, "emu/hda/efi/boot"
		end
	end
end

task :deps => :deps_other do
	run *%w{cargo build --manifest-path vendor/rust/src/src/libcore/Cargo.toml --features disable_float --target x86_32-avery-kernel -v}
	run *%w{cargo build --manifest-path kernel/arch/x64/multiboot/Cargo.toml --target x86_32-avery-kernel -v}

	build = Build.new('build', 'info.yml')
	build.run do
		mkdirs("build/phase")

		# Build assembly plugin
		run *%w{rustc -O --out-dir build/phase vendor/asm/assembly.rs}

		# Build 64-bit libcore
		build_libcore(build, "", %w{--target x86_64-avery-kernel})

		# Build custom 64-bit libstd for the kernel
		run 'rustc', '-L', 'build/crates', *RUSTFLAGS, '--target', 'x86_64-avery-kernel', 'kernel/std/std.rs', '--out-dir', build.output("crates")

		# Build 32-bit libcore
		build_libcore(build, "bootstrap", %w{--target x86_32-avery-kernel})

		# Build 32-bit multiboot bootstrap code
		build_crate(build, "bootstrap", "bootstrap", %w{--target x86_32-avery-kernel}, 'kernel/arch/x64/multiboot/bootstrap.rs', ['--emit=asm,llvm-ir'])

		# Place 32-bit bootstrap code into a 64-bit ELF

		asm = build.output("bootstrap/bootstrap.s")

		File.open(asm, 'r+') do |file|
			content = file.read.lines.map do |l|
				if l.strip =~ /^.cfi.*/
					""
				else
					l
				end
			end.join
			file.pos = 0
			file.truncate 0
			file.write ".code32\n"
			file.write content
		end

		run AS, asm, '-o', kernel_object_bootstrap

		# Strip all but the entry symbol `setup_long_mode` so they don't conflict with 64-bit kernel symbols
		run 'x86_64-elf-objcopy', '--strip-debug', '-G', 'setup_long_mode', kernel_object_bootstrap
	end
end

task :build do
	type = :multiboot
	build_kernel.call
end

task :build_efi do
	type = :efi
	build_kernel.call
end

task :vmware do
	Dir.chdir('emu/') do
		run *%W{#{QEMU_PATH}qemu-img convert -O vmdk grubdisk.img avery.vmdk}
	end
end

def run_qemu(efi)
	Dir.chdir('emu/') do
		puts "Running QEMU..."
		FileUtils.rm("serial.txt") if File.exists?("serial.txt")
		FileUtils.rm("int.log") if File.exists?("int.log")
		opts = if efi
			%w{-L . -bios OVMF.fd -hda fat:hda}
		else
			%w{-L qemu/Bios -drive file=grubdisk.img,index=0,media=disk,format=raw}
		end
		 # -d ,cpu_reset
		 run *(%W{#{QEMU_PATH}qemu-system-x86_64 -serial file:serial.txt -d int -D int.log -no-reboot -s -smp 4} + opts)
	end
end

task :qemu => :build do
	run_qemu(false)
end

task :qemu_efi => :build_efi do
	run_qemu(true)
end

task :bochsdbg => :build do
	Dir.chdir('emu/') do
		puts "Running Bochs..."
		run 'bochs\bochsdbg', '-q', '-f', 'avery.bxrc'
	end
end

task :bochs4 => :build do

	Dir.chdir('emu/') do
		puts "Running Bochs..."
		run 'bochs4\bochs', '-q', '-f', 'avery4.bxrc' # bochs4\bochs -q -f avery4.bxrc
	end
end

task :bochs => :build do

	Dir.chdir('emu/') do
		puts "Running Bochs..."
		run 'bochs\bochs', '-q', '-f', 'avery.bxrc'
	end
end

CORES = 4

build_type = :build

# Build a unix like package at src
build_unix_pkg = proc do |src, opts, &proc|
	if build_type == :clean
		FileUtils.rm_rf(["built", "configured", "build", "install"])
		FileUtils.rm_rf(["src/target"]) if opts[:cargo]
	end

	next if build_type != :build

	old_env = ENV.to_h
	ENV.replace(CLEANENV.merge(opts[:env] || {}))

	mkdirs("install")
	prefix = File.realpath("install");

	build_dir = opts[:intree] ? src : "build"

	mkdirs(build_dir)

	unless File.exists?("configured")
		Dir.chdir(build_dir) do
			old_unix = UNIX_EMU[0]
			UNIX_EMU[0] = opts[:unix]
			proc.call(File.join("..", src), prefix)
			UNIX_EMU[0] = old_unix
		end
		run 'touch', "configured"
	end if proc

	unless File.exists?("built")
		bin_path = "install"

		Dir.chdir(build_dir) do
			if src == 'rust' && RUSTSHORT
				bin_path = "#{ARCH}/stage1"
				run "make", "rustc-stage1", "-j#{CORES}"
			else
				if opts[:cargo]
					run "cargo", "install", "--path=#{File.join("..", src)}", "--root=#{File.join("..", 'install')}"
				else
					if opts[:ninja] && NINJA
						run "ninja"
						run "ninja", "install"
					else
						old_unix = UNIX_EMU[0]
						UNIX_EMU[0] = opts[:unix]
						run "make", "-j#{CORES}"
						run "make", "install"
						UNIX_EMU[0] = old_unix
					end
				end
			end
		end

		# Copy dependencies from MSYS
		if opts[:unix] && File.exists?('/usr/bin/msys-2.0.dll')
			mkdirs("#{bin_path}/bin")
			run 'cp', '/usr/bin/msys-2.0.dll', "#{bin_path}/bin/msys-2.0.dll"
		end

		run 'touch', "built"
	end

	ENV.replace(old_env)
end

# Build a unix like package from url
build_from_url = proc do |url, name, ver, opts = {}, &proc|
	src = "#{name}-#{ver}"
	ext = opts[:ext] || "bz2"
	path = opts[:path] || name

	mkdirs(path)
	Dir.chdir(path) do
		mkdirs("install")
		prefix = File.realpath("install");

		if !File.exists?(src) && build_type != :clean
			tar = "#{src}.tar.#{ext}"
			unless File.exists?(tar)
				run 'curl', '-O', "#{url}#{tar}"
			end

			uncompress = case ext
				when "bz2"
					"j"
				when "xz"
					"J"
				when "gz"
					"z"
			end

			run 'tar', "-#{uncompress}xf", tar
		end

		build_unix_pkg.(src, opts, &proc)
	end
end

checkout_git = proc do |path, url, opts = {}, &proc|
	branch = opts[:branch] || "master"
	if Dir.exists?(File.join(path, ".git"))
		if build_type == :clean
			#run "git", "clean", "-dfx"
		end

		if build_type == :update
			Dir.chdir(path) do
				rep = Dir.pwd
				git_url = url.gsub("https://", "git@").sub("/", ':')
				remote = `git remote get-url origin`.strip
				if remote != url && remote != git_url
					raise "Git remote mismatch for #{rep}. Local is #{remote}. Required is #{url}"
				end
				unless `git status -uno --porcelain`.strip.empty?
					raise "Dirty working directory in  #{rep}"
				end
				local = `git rev-parse #{branch}`
				remote = `git rev-parse origin/#{branch}`
				raise "Local branch #{branch} doesn't match origin in #{rep}" if local != remote
				run "git", "fetch", "origin"
				run "git", "checkout", branch
				run "git", "reset", "--hard", "origin/#{branch}"
				new_local = `git rev-parse #{branch}`
				if local != new_local
					puts "Must rebuild #{rep}"
					next :rebuild
				end
			end
		end
	else
		if build_type == :build
			b = opts[:branch] ? ["-b",  opts[:branch]] : []
			run "git", "clone", *b, url, path
		end
	end

	nil
end

# Build a unix like package from git
build_from_git = proc do |name, url, opts = {}, &proc|
	mkdirs(name)
	Dir.chdir(name) do
		if checkout_git.("src", url, opts) == :rebuild
			old = build_type
			build_type = :clean
			build_unix_pkg.("src", opts, &proc)
			build_type = old
		end
		build_unix_pkg.("src", opts, &proc)
	end
end

update_cfg = proc do |path|
	run 'cp', File.join(AVERY_DIR, 'vendor/config.guess'), path
	run 'cp', File.join(AVERY_DIR, 'vendor/config.sub'), path
end

task :deps_msys do
	raise "Cannot install dependencies when not in a MSYS2 MINGW shell" if !ON_WINDOWS_MINGW
	mingw = if ENV['MSYSTEM']='MINGW64'
		'mingw-w64-x86_64'
	else
		'mingw-w64-i686'
	end
	run *%W{pacman --needed --noconfirm -S ruby git tar gcc bison make texinfo patch diffutils autoconf #{mingw}-python2 #{mingw}-cmake #{mingw}-gcc #{mingw}-ninja}
end

external_builds = proc do |real, extra|
	raise "Cannot build non-UNIX dependencies with MSYS2 shell, use the MinGW shell and run `rake deps`" if ON_WINDOWS && !ON_WINDOWS_MINGW
	raise "Ninja is required on Windows" if ON_WINDOWS_MINGW && !NINJA

	mkdirs('emu')
	Dir.chdir('emu/') do
		if ON_WINDOWS && QEMU_PATH == 'qemu/' && !Dir.exists?('qemu')
			run 'curl', '-O', 'https://raw.githubusercontent.com/AveryOS/binaries/master/qemu.tar.xz'
			run 'tar', "Jxf", 'qemu.tar.xz'
			FileUtils.rm('qemu.tar.xz')
		end

		unless File.exists?('grubdisk.img')
			run 'curl', '-O', 'https://raw.githubusercontent.com/AveryOS/binaries/master/disk.tar.xz'
			run 'tar', "Jxf", 'disk.tar.xz'
			FileUtils.rm('disk.tar.xz')
		end
	end

	Dir.chdir('vendor/') do
		unless Dir.exists?("rlibc/src")
			run "git", "clone" , "https://github.com/alexcrichton/rlibc.git", "rlibc/src"
		end

		build_from_url.("ftp://ftp.gnu.org/gnu/binutils/", "binutils", "2.25", {unix: true, path: 'elf-binutils'}) do |src, prefix|
			run File.join(src, 'configure'), "--prefix=#{prefix}", *%w{--target=x86_64-elf --with-sysroot --disable-nls --disable-werror}
		end # binutils is buggy with mingw-w64

		build_from_url.("ftp://ftp.gnu.org/gnu/mtools/", "mtools", "4.0.18", {unix: true}) do |src, prefix|
			update_cfg.(src)
			#run 'cp', '-rf', "../../libiconv/install", ".."
			Dir.chdir(src) do
				run 'patch', '-i', "../../mtools-fix.diff"
			end
			opts = []
			opts += ["LIBS=-liconv"] if Gem::Platform::local.os == 'darwin'
			run File.join(src, 'configure'), "--prefix=#{prefix}", *opts
		end # mtools doesn't build with mingw-w64

		checkout_git.("llvm", "https://github.com/AveryOS/llvm.git", {branch: "avery"})
		mkdirs("llvm/src/tools")
		checkout_git.("llvm/src/tools/clang", "https://github.com/AveryOS/clang.git", {branch: "avery"})

		build_from_git.("llvm", "https://github.com/AveryOS/llvm.git", {branch: "avery", ninja: true}) do |src, prefix|
			#-DBUILD_SHARED_LIBS=On  rustc on OS X wants static
			opts = %W{-DLLVM_TARGETS_TO_BUILD=X86 -DCMAKE_BUILD_TYPE=RelWithDebInfo -DCMAKE_INSTALL_PREFIX=#{prefix}}
			opts += ['-G',  'Ninja', '-DLLVM_PARALLEL_LINK_JOBS=1'] if NINJA
			opts += %w{-DCMAKE_CXX_COMPILER=g++ -DCMAKE_C_COMPILER=gcc -DBUILD_SHARED_LIBS=On} if ON_WINDOWS_MINGW
			run "cmake", src, *opts
		end

		build_from_url.("ftp://ftp.gnu.org/gnu/libiconv/", "libiconv", "1.14", {unix: true, ext: "gz"}) do |src, prefix|
			run File.join(src, 'configure'), "--prefix=#{prefix}"
		end if nil #RbConfig::CONFIG['host_os'] == 'msys'

		# autotools for picky newlib

		build_from_url.("ftp://ftp.gnu.org/gnu/automake/", "automake", "1.12", {unix: true, ext: "gz"}) do |src, prefix|
			update_cfg.(src)
			update_cfg.(File.join(src, 'lib'))
			run File.join(src, 'configure'), "--prefix=#{prefix}"
		end if extra

		build_from_url.("ftp://ftp.gnu.org/gnu/autoconf/", "autoconf", "2.65", {unix: true, ext: "gz"}) do |src, prefix|
			update_cfg.(src)
			run File.join(src, 'configure'), "--prefix=#{prefix}"
		end if extra

		build_from_git.("binutils", "https://github.com/AveryOS/binutils.git", {branch: "avery", unix: true}) do |src, prefix|
			run File.join(src, 'configure'), "--prefix=#{prefix}", *%w{--target=x86_64-pc-avery --with-sysroot --disable-nls --disable-werror --disable-gdb --disable-sim --disable-readline --disable-libdecnumber}
		end if extra # binutils is buggy with mingw-w64

		build_from_git.("newlib", "https://github.com/AveryOS/newlib.git", {branch: "avery"}) do |src, prefix|
			run File.join(src, 'configure'), "--prefix=#{prefix}", "--target=x86_64-pc-avery", 'CC_FOR_TARGET=clang -fno-integrated-as -ffreestanding --target=x86_64-pc-avery -ccc-gcc-name x86_64-pc-avery-gcc'
		end if extra

		# C++ isn't working yet
		#run "rm", "llvm/install/bin/clang++#{EXE_POST}"

		if real
			run "rm", "-rf", "sysroot"
			mkdirs('sysroot')
			#run 'cp', '-r', 'avery-binutils/install/x86_64-pc-avery/.', "sysroot/usr/"
			run 'cp', '-r', 'avery-newlib/install/x86_64-pc-avery/.', "sysroot" if Dir.exists?("avery-newlib/install")
		end

		# CMAKE_STAGING_PREFIX, CMAKE_INSTALL_PREFIX

		build_from_git.("libcxx", "http://llvm.org/git/libcxx.git") do |src, prefix| # -nodefaultlibs
			opts = ["-DLLVM_CONFIG_PATH=#{File.join(src, "../../llvm/install/bin/llvm-config")}", "-DCMAKE_TOOLCHAIN_FILE=../../toolchain.txt", "-DCMAKE_STAGING_PREFIX=#{prefix}"]
			opts += ['-G',  'MSYS Makefiles'] if ON_WINDOWS_MINGW
			run "cmake", src, *opts
		end if nil

		build_from_git.("compiler-rt", "http://llvm.org/git/compiler-rt.git") do |src, prefix|
			opts = ["-DLLVM_CONFIG_PATH=#{File.join(src, "../../llvm/install/bin/llvm-config")}", "-DCMAKE_TOOLCHAIN_FILE=../../toolchain.txt", "-DCMAKE_STAGING_PREFIX=#{prefix}", "-DCMAKE_INSTALL_PREFIX=#{prefix}", "-DCOMPILER_RT_BUILD_SANITIZERS=Off"]
			opts += ['-G',  'MSYS Makefiles'] if ON_WINDOWS_MINGW
			run "cmake", src, *opts
		end if nil

		#ENV['VERBOSE'] = '1'

		# clang is not the host compiler, force use of gcc
		env = {'CC' => 'gcc', 'CXX' => 'g++'}
		build_from_git.("rust", "https://github.com/AveryOS/rust.git", {branch: "avery", env: env}) do |src, prefix|
			run File.join(src, 'configure'), "--enable-debuginfo", "--prefix=#{prefix}", "--llvm-root=#{File.join(src, "../../llvm/build")}", "--disable-docs", "--target-sysroot=#{File.join(Dir.pwd, "../../sysroot")}"#, "--target=x86_64-pc-avery", "--disable-jemalloc"
		end

		build_from_git.("cargo", "https://github.com/brson/cargo.git", {intree: true, branch: 'rustflags'}) do |src, prefix|
			Dir.chdir(src) do
				run *%w{git submodule update --init}
			end
			run File.join(src, 'configure'), "--enable-nightly", "--prefix=#{prefix}", "--local-rust-root=#{File.expand_path("../vendor/rust/install", __FILE__)}"
		end

		# place compiler-rt in lib/rustlib/x86_64-pc-avery/lib - rustc links to it // clang links to it instead
		#run 'cp', '-r', 'libcompiler-rt.a', "sysroot/lib" if real && File.exists?("libcompiler-rt.a")

		env = {'LIBCLANG_PATH' => File.expand_path("../vendor/llvm/install/#{ON_WINDOWS ? 'bin' : 'lib'}", __FILE__)}
		build_from_git.("bindgen", "https://github.com/crabtw/rust-bindgen.git", {cargo: true, env: env}) if extra
	end
end

task :deps_other do
	external_builds.(true, false)
end

task :extra do
	external_builds.(true, true)
end

task :update do
	build_type = :update
	external_builds.(false, true)
end

task :update_all => :update do
	#checkout_git.(".", "https://github.com/AveryOS/avery.git")
end

task :clean do
	build_type = :clean
	external_builds.(false, true)
end

task :user do
	build = Build.new('build', 'info.yml')
	build.run do
		build_libcore(build, "user", %w{--target x86_64-pc-avery})
		# rustc can't pass linker arguments with spaces in them; require a space free path
		build_crate(build, "user", "user", %w{--target x86_64-pc-avery}, 'user/test.rs', ['-Z', 'print-link-args', '-C', "link-args=-v --sysroot #{File.expand_path('../vendor/sysroot', __FILE__)}"])
	end
end

task :sh do
	run 'sh'
end

task :default => :build
