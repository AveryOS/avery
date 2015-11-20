require 'fileutils'
require_relative 'rake/build'
require_relative 'rake/lokar'

ENV['RUST_TARGET_PATH'] = File.expand_path('../targets', __FILE__)
ENV['RUST_BACKTRACE'] = '1'

def mkdirs(target)
	FileUtils.makedirs(target)
end

def run(*cmd)
	puts cmd.join(" ")
	system([cmd.first, cmd.first], *cmd[1..-1])
	raise "Command #{cmd.join(" ")} failed with error code #{$?}" if $? != 0
end

raise "Install and use MSYS2 Ruby" if ENV['MSYSTEM'] && Gem.win_platform?

ON_WINDOWS = Gem.win_platform? || ENV['MSYSTEM']

EXE_POST = ON_WINDOWS ? ".exe" :	""

def append_path(path)
	if Gem.win_platform?
		ENV['PATH'] = "#{path.gsub('/', '\\')};#{ENV['PATH']}"
	else
		ENV['PATH'] = "#{path}:#{ENV['PATH']}"
	end
end

append_path(File.expand_path('../vendor/binutils/install/bin', __FILE__))
append_path(File.expand_path('../vendor/mtools/install/bin', __FILE__))

QEMU_PATH = "#{'qemu/' if ON_WINDOWS}"
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
#--llvm-args=--inline-threshold=0
RUSTFLAGS = ['-C',"ar=x86_64-elf-ar", '--sysroot', File.expand_path('../build/sysroot', __FILE__)] +
	%w{-C opt-level=1 -C debuginfo=1 -Z no-landing-pads}

def build_libcore(build, crate_prefix, flags)
	crates = build.output(File.join(crate_prefix, "crates"))
	mkdirs(crates)
	run 'rustc', *RUSTFLAGS, *flags, 'vendor/rust/rust/src/libcore/lib.rs', '--out-dir', crates

	# libcore needs rlibc
	run 'rustc', '-L', crates, *RUSTFLAGS, *flags, '--crate-type', 'rlib', '--crate-name', 'rlibc', 'vendor/rlibc/src/lib.rs', '--out-dir', crates
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
	kernel_assembly_bootstrap = build.output "#{type}/bootstrap.s"

	sources = build.package('src/**/*')

	efi_files = sources.extract('src/arch/x64/efi/**/*')
	multiboot_files = sources.extract('src/arch/x64/multiboot/**/*')

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
		build_crate(build, "", "#{type}", %w{--target x86_64-avery-kernel}, 'src/kernel.rs', flags)

		# Preprocess files

		gen_folder = "gen/#{type}"

		linker_script = "src/arch/x64/kernel.ld"
		generated_files = ['src/arch/x64/interrupts.s', linker_script]

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

task :deps_base do
	build = Build.new('build', 'info.yml')
	build.run do
		mkdirs("build/phase")

		# Build assembly plugin
		run *%w{rustc -O --out-dir build/phase vendor/asm/assembly.rs}

		# Build 64-bit libcore
		build_libcore(build, "", %w{--target x86_64-avery-kernel})

		# Build custom 64-bit libstd for the kernel
		run 'rustc', '-L', 'build/crates', *RUSTFLAGS, '--target', 'x86_64-avery-kernel', 'src/std/std.rs', '--out-dir', build.output("crates")

		# Build 32-bit libcore
		build_libcore(build, "bootstrap", %w{--target x86_32-avery-kernel})

		# Build 32-bit multiboot bootstrap code
		build_crate(build, "bootstrap", "bootstrap", %w{--target x86_32-avery-kernel}, 'src/arch/x64/multiboot/bootstrap.rs', ['--emit=asm,llvm-ir'])

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
			%w{-L qemu/Bios -hda grubdisk.img}
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

CORES = 3

# Build a unix like package at src
build_unix_pkg = proc do |src, &proc|
	mkdirs("install")
	prefix = File.realpath("install");

	mkdirs("build")

	unless File.exists?("configured")
		Dir.chdir("build") do
			proc.call(File.join("..", src), prefix)
		end
		run 'touch', "configured"
	end

	unless File.exists?("built")
		Dir.chdir("build") do
			run "make", "-j#{CORES}"
			run "make", "install"
		end

		# Copy dependencies from MSYS/Cygwin
		if File.exists?('/usr/bin/msys-2.0.dll')
			mkdirs("install/bin")
			run 'cp', '/usr/bin/msys-2.0.dll', "install/bin/msys-2.0.dll"
		end

		run 'touch', "built"
	end
end

# Build a unix like package from url
build_from_url = proc do |url, name, ver, ext = "bz2", &proc|
	src = "#{name}-#{ver}"

	mkdirs(name)
	Dir.chdir(name) do
		mkdirs("install")
		prefix = File.realpath("install");

		unless File.exists?(src)
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

			run 'tar', "#{uncompress}xf", tar
		end

		build_unix_pkg.(src, &proc)
	end
end

# Build a unix like package from git
build_from_git = proc do |name, url, &proc|
	mkdirs(name)
	Dir.chdir(name) do
		unless Dir.exists?(name)
			run "git", "clone" , url, name
		end
		build_unix_pkg.(name, &proc)
	end
end

task :deps_unix do
	raise "Cannot build UNIX dependencies with MinGW" if ENV['MSYSTEM'] && ENV['MSYSTEM'].start_with?('MINGW')

	Dir.chdir('vendor/') do
		build_from_url.("ftp://ftp.gnu.org/gnu/binutils/", "binutils", "2.25") do |src, prefix|
			run File.join(src, 'configure'), "--prefix=#{prefix}", *%w{--target=x86_64-elf --with-sysroot --disable-nls --disable-werror}
		end # binutils is buggy with mingw-w64

		build_from_url.("ftp://ftp.gnu.org/gnu/libiconv/", "libiconv", "1.14", "gz") do |src, prefix|
			run File.join(src, 'configure'), "--prefix=#{prefix}"
		end if nil #RbConfig::CONFIG['host_os'] == 'msys'

		build_from_url.("ftp://ftp.gnu.org/gnu/mtools/", "mtools", "4.0.18") do |src, prefix|
			#run 'cp', '-rf', "../../libiconv/install", ".."
			Dir.chdir(src) do
				# mtools can't detect MSYS2, fix this
				if ENV['MSYSTEM']
					run 'cp', '/usr/share/libtool/build-aux/config.guess', 'config.guess'
					run 'cp', '/usr/share/libtool/build-aux/config.sub', 'config.sub'
				end
				run 'patch', '-i', "../../mtools-fix.diff"
			end
			run File.join(src, 'configure'), "--prefix=#{prefix}"#, "LIBS=-liconv"
		end# mtools doesn't build with mingw-w64

		build_from_git.("avery-binutils", "https://github.com/Zoxc/avery-binutils.git") do |src, prefix|
			run File.join(src, 'configure'), "--prefix=#{prefix}", *%w{--target=x86_64-pc-avery --with-sysroot --disable-nls --disable-werror --disable-gdb --disable-sim --disable-readline --disable-libdecnumber}
		end # binutils is buggy with mingw-w64
	end
end

task :deps => :deps_base do
	Dir.chdir('emu/') do
		unless File.exists?('grubdisk.img')
			run 'curl', '-O', unknown_URL
			run 'tar', "Jxf", 'disk.tar.xz'
			FileUtils.rm('disk.tar.xz')
		end
	end

	Dir.chdir('vendor/') do
		unless Dir.exists?("rust")
			run "git", "clone" , 'https://github.com/rust-lang/rust.git', 'rust'
		end

		unless Dir.exists?("rlibc")
			run "git", "clone" , "https://github.com/rust-lang/rlibc.git"
		end
	end
end

task :deps_user do
	Dir.chdir('vendor/') do
		build_from_git.("avery-llvm", "https://github.com/Zoxc/llvm-sfi.git") do |src, prefix|
			Dir.chdir(File.join(src, 'tools')) do
					unless Dir.exists?("clang")
						run "git", "clone" , "http://llvm.org/git/clang.git"
					end
			end
			opts = %W{-DBUILD_SHARED_LIBS=On -DLLVM_TARGETS_TO_BUILD=X86 -DCMAKE_BUILD_TYPE=RelWithDebInfo -DCMAKE_INSTALL_PREFIX=#{prefix}}
			opts += ['-G',  'MSYS Makefiles'] if ON_WINDOWS
			run "cmake", src, *opts
		end

		append_path(File.realpath("avery-llvm/install/bin"))
		append_path(File.realpath("avery-llvm/install/lib")) # LLVM places DLLs in /lib ...
		append_path(File.realpath("avery-binutils/install/bin"))

		build_from_git.("avery-newlib", "https://github.com/Zoxc/avery-newlib.git") do |src, prefix|
			run File.join(src, 'configure'), "--prefix=#{prefix}", "--target=x86_64-pc-avery", 'CC_FOR_TARGET=clang -ffreestanding --target=x86_64-pc-avery -ccc-gcc-name x86_64-pc-avery-gcc'
		end

		build_from_git.("avery-rust", "https://github.com/rust-lang/rust.git") do |src, prefix|
			run File.join(src, 'configure'), "--prefix=#{prefix}", "--target=x86_64-pc-avery", "--llvm-root=#{File.join(src, "../../avery-llvm/build")}", "--disable-docs", "--disable-jemalloc"
		end
	end
end

task :match_rustc do
	rustc_ver = /\((.*?) /.match(`rustc --version`)[1]

	Dir.chdir('vendor/') do
		Dir.chdir('rlibc/') do
			run *%w{git pull origin master}
		end

		Dir.chdir('rust/') do
			run *%w{git checkout master}
			run *%w{git pull origin master}
			run *%w{git checkout}, rustc_ver
		end
	end
end

task :default => :build
