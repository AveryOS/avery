require 'fileutils'
require 'digest'
require_relative 'rake/build'
require_relative 'rake/lokar'

START_TIME = Time.new

MACOS = /darwin/ =~ RUBY_PLATFORM

CURRENT_DIR = Rake.original_dir
AVERY_DIR = File.expand_path('../', __FILE__)
Dir.chdir(AVERY_DIR)

def path(p)
	File.expand_path(File.join(AVERY_DIR, p))
end

def so_append_path(path)
	if Gem.win_platform?
		ENV['PATH'] = "#{path.gsub('/', '\\')};#{ENV['PATH']}"
	else
		ENV['PATH'] = "#{path}:#{ENV['PATH']}"
	end
end

def append_path(path)
	if Gem.win_platform?
		ENV['PATH'] = "#{path.gsub('/', '\\')};#{ENV['PATH']}"
	else
		ENV['PATH'] = "#{path}:#{ENV['PATH']}"
	end
end

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

CC = (which(ENV['CC']) if ENV['CC'])
CXX = (which(ENV['CXX']) if ENV['CXX'])

def pkg_path(name, type, *ext)
	path(File.join('build/pkgs', type, name, *ext))
end

append_path(File.expand_path('../build/cargo/home/bin', __FILE__))
append_path(path('build/pkgs/install/cmake/bin'))
append_path(path('build/pkgs/install/elf-binutils/bin'))
append_path(path('build/pkgs/install/mtools/bin'))
append_path(path('build/pkgs/install/llvm/bin'))
append_path(path('build/pkgs/install/binutils/bin'))
append_path(path('build/pkgs/install/cargo/bin'))
append_path(path('build/pkgs/install/rust/bin'))
append_path(path('build/pkgs/install/autoconf/bin'))
append_path(path('build/pkgs/install/automake/bin'))
append_path(path('build/pkgs/install/bindgen/bin'))
append_path(File.expand_path("../util/rlib_ir/target/debug", __FILE__))

sos = pkg_path('llvm', 'install', 'lib') + ":" + pkg_path('rust', 'install', 'lib')
ENV['DYLD_LIBRARY_PATH'] = sos
ENV['LD_LIBRARY_PATH'] = sos

ENV['RUST_BACKTRACE'] = '1'
ENV['RUST_NEW_ERROR_FORMAT'] = 'true'

# rustc build needs LLVM in PATH on Windows
CLEANENV = ENV.to_hash

def new_env(k, v)
	old = ENV[k]
	ENV[k] = v
	yield
	ENV[k] = old
end

def hostpath(p)
	if ENV['MSYSTEM']
		`cygpath -wm #{File.expand_path(p).shellescape}`.chomp
	else
		File.expand_path(p)
	end
end

ENV['CARGO_HOME'] = hostpath('build/cargo/home')
ENV['CARGO_TARGET_DIR'] = File.expand_path('../build/cargo/target', __FILE__)
ENV['RUST_TARGET_PATH'] = File.expand_path('../targets', __FILE__)
UNIX_EMU = [false]

def mkdirs(target)
	FileUtils.makedirs(target)
end

def run_stay(*cmd)
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
end

def run(*cmd)
	run_stay(*cmd)
	raise "Command #{cmd.join(" ")} failed with error code #{$?}" if $? != 0
end

raise "Install and use MSYS2 Ruby" if ENV['MSYSTEM'] && Gem.win_platform?

ON_WINDOWS = Gem.win_platform? || ENV['MSYSTEM']

ON_WINDOWS_MINGW = ENV['MSYSTEM'] && ENV['MSYSTEM'].start_with?('MINGW')

raise "Cannot build non-UNIX dependencies with MSYS2 shell, use the MinGW shell and run `rake`" if ON_WINDOWS && !ON_WINDOWS_MINGW

def ninja
	ninja = which('ninja')
	raise "Ninja is required on Windows" if ON_WINDOWS_MINGW && !ninja
	ninja
end

EXE_POST = ON_WINDOWS ? ".exe" :	""

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

ENV['AVERY_BUILD'] = 'RELEASE' unless ENV['AVERY_BUILD']

RELEASE_BUILD = ENV['AVERY_BUILD'] == 'RELEASE' ? true : false
CARGO_BUILD = RELEASE_BUILD ? 'release' : 'debug'

RUSTFLAGS = ['--sysroot', hostpath('build/sysroot')] + %w{-Z force-overflow-checks=on -Z orbit -C panic=abort -C llvm-args=-inline-threshold=0 -C debuginfo=1 -C target-feature=-mmx,-sse,-sse2}

# Workaround bug with LLVM linking
ENV['RUSTFLAGS_HOST'] = "-L #{hostpath(pkg_path("llvm", 'install', ON_WINDOWS ? "bin" : "lib"))}"

def cargo(path, target, cargoflags = [], flags = [], rustflags = nil)
	cargoflags += ['--target', target]
		cargoflags += ['-j', '1'] if CORES == 1
	if target == 'x86_64-pc-avery'
		ENV['RUSTFLAGS'] = (rustflags || []).join(" ")
	else
		ENV['RUSTFLAGS'] = (rustflags || RUSTFLAGS).join(" ")
	end
	puts "RUSTFLAGS = #{ENV['RUSTFLAGS']}"
	run 'cargo', 'rustc', *(RELEASE_BUILD ? ['--release'] : []), '--manifest-path', File.join(path, 'Cargo.toml'), *cargoflags, '--', *flags
end

kernel_object_bootstrap = "build/bootstrap.o"

type = :multiboot
build_kernel = proc do |skip = false|
	build = Build.new('build', 'info.yml')
	mkdirs(build.output "#{type}")
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
		flags = ['-C', 'lto', "--emit=llvm-ir,obj=#{kernel_object}"]
		flags += ['--cfg', 'multiboot'] if type == :multiboot

		cargo('kernel', 'x86_64-avery-kernel', [], flags) unless skip

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

		compiler_rt = pkg_path("compiler-rt/x86_64-unknown-unknown-elf", 'install', 'lib/generic/libclang_rt.builtins-x86_64.a')

		# Finally link
		run 'x86_64-elf-ld', '-z', 'max-page-size=0x1000',
			'-T', build.output(File.join(gen_folder, linker_script)),
			*objects, compiler_rt,
			'-o', kernel_binary
	end
end

task :std do
	rebuild("user-sysroot", ["rust", "llvm"]) do
		run "rm", "-rf", "build/cargo/avery-sysroot-target"
		ENV['CC_x86_64-pc-avery'] = 'clang --target=x86_64-pc-avery'
		ENV['CXX_x86_64-pc-avery'] = 'clang++ --target=x86_64-pc-avery'
		ENV['CPP_x86_64-pc-avery'] = 'clang --target=x86_64-pc-avery'
		ENV['AR_x86_64-pc-avery'] = 'x86_64-pc-avery-ar'

		new_env('CARGO_TARGET_DIR', 'build/cargo/avery-sysroot-target') do
			sysroot = "build/cargo/avery-sysroot-target/x86_64-pc-avery/debug/deps"

			ENV['RUSTFLAGS'] = '-C llvm-args=-inline-threshold=0 -Z orbit --sysroot vendor/fake-sysroot -Z force-overflow-checks=on -C opt-level=2'
			run *%w{cargo build -j 1 --target x86_64-pc-avery --manifest-path vendor/cargo-sysroot/Cargo.toml --verbose}
			dir = "#{pkg_path('rust', 'install')}/lib/rustlib/x86_64-pc-avery/lib"
			FileUtils.rm_rf([dir])
			mkdirs(dir)
			run 'cp', '-r', "#{sysroot}/.", dir
		end
	end
end

task :deps => [:user, :deps_other] do
	rebuild("kernel-sysroots", ["rust"], CARGO_BUILD) do
		run "rm", "-rf", "build/cargo/sysroot-target"
		new_env('CARGO_TARGET_DIR', 'build/cargo/sysroot-target') do
			run "rm", "-rf", "build/sysroot"

			build_sysroot = proc do |target|
				cargo "sysroot", target
				mkdirs("build/sysroot/lib/rustlib/#{target}/lib")
				run 'cp', '-r', "build/cargo/sysroot-target/#{target}/#{CARGO_BUILD}/deps/.", "build/sysroot/lib/rustlib/#{target}/lib"
			end

			build_sysroot.('x86_32-avery-kernel')
			build_sysroot.('x86_64-avery-kernel')
		end
	end

	cargo 'kernel/arch/x64/multiboot', 'x86_32-avery-kernel', [], (%w{-C lto --emit=obj=build/bootstrap-32.o})

	compiler_rt = pkg_path("compiler-rt/x86_64-unknown-unknown-elf", 'install', 'lib/generic/libclang_rt.builtins-i386.a')

	build = Build.new('build', 'info.yml')
	build.run do
		build.process kernel_object_bootstrap, "build/bootstrap-32.o" do |o, i|
			# Link in compiler-rt
			run 'x86_64-elf-ld', '-melf_i386', '-r', i, compiler_rt, '-o', "build/bootstrap-32rt.o"

			# Place 32-bit bootstrap code into a 64-bit ELF
			run 'x86_64-elf-objcopy', '-O', 'elf64-x86-64', "build/bootstrap-32rt.o", kernel_object_bootstrap

			# Strip all but the entry symbol `setup_long_mode` so they don't conflict with 64-bit kernel symbols
			run 'x86_64-elf-objcopy', '--strip-debug', '-G', 'setup_long_mode', kernel_object_bootstrap
		end
	end
end

task :kernel do
	type = :multiboot
	build_kernel.call
end

task :build => :deps do
	type = :multiboot
	build_kernel.call
end

task :build_efi => :deps do
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

task :emu do
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

	kernel_binary = "build/#{type}/kernel.elf"

	# Copy kernel into emulation environment
	case type
		when :multiboot
			run 'mcopy', '-D', 'o', '-D', 'O', '-i' ,'emu/grubdisk.img@@1M', kernel_binary, '::kernel.elf'
		when :boot
			FileUtils.cp kernel_binary, "emu/hda/efi/boot"
	end
end

task :uqemu => [:user_skip] do
	type = :multiboot
	build_kernel.call(true)
	Rake::Task["emu"].invoke
	run_qemu(false)
end

task :fqemu => [:kernel, :emu] do
	run_qemu(false)
end

task :qemu => [:build, :emu] do
	run_qemu(false)
end

task :qemu_efi => [:build_efi, :emu] do
	run_qemu(true)
end

task :bochsdbg => [:build, :emu] do
	Dir.chdir('emu/') do
		puts "Running Bochs..."
		run 'bochs/bochsdbg', '-q', '-f', 'avery.bxrc'
	end
end

task :bochs4 => [:build, :emu] do
	Dir.chdir('emu/') do
		puts "Running Bochs..."
		run 'bochs4/bochs', '-q', '-f', 'avery4.bxrc'
	end
end

task :bochs => [:build, :emu] do
	Dir.chdir('emu/') do
		puts "Running Bochs..."
		run 'bochs/bochs', '-q', '-f', 'avery.bxrc'
	end
end

require_relative 'rake/deps'

CORES = ENV['TRAVIS'] ? 2 : 4
#CORES = 1

task :deps_other do
	EXTERNAL_BUILDS.(:build, true, false)
end

task :fmt do
	run 'cargo', 'install', 'rustfmt' unless which 'rustfmt'
	format = proc { run 'cargo', 'fmt', '--', '--config-path', File.expand_path('.') }
	Dir.chdir('kernel') { format.() }
	Dir.chdir('kernel/arch/x64/multiboot') { format.() }
end

build_user = proc do
	ENV['RUSTFLAGS'] = nil

	cargo 'user', 'x86_64-pc-avery', ['--verbose'], ['--emit=llvm-ir']
	cargo 'user/hello', 'x86_64-pc-avery'

	mkdirs("build/user")
	FileUtils.cp "build/cargo/target/x86_64-pc-avery/#{CARGO_BUILD}/hello", "build/user"

	#run *%w{clang --target=x86_64-pc-avery user/hello/hello.c -o build/user/hello}

	build = Build.new('build', 'info.yml')
	build.run do
		build.process  "build/user/hello.o", "build/user/hello" do |o, i|
			run 'x86_64-elf-objcopy', '-I', 'binary', '-B', 'i386', '-O', 'elf64-x86-64', i, o
		end
	end
end

task :user_skip do
	build_user.()
end
task :user => [:deps_other, :std, :user_skip]

task :sh do
	ENV.replace(CLEANENV)
	Dir.chdir(CURRENT_DIR)
	run 'bash'
end

task :verifier => [:dep_capstone] do
	run 'cargo', 'install', 'rustfmt' unless which('rustfmt')

	Dir.chdir("verifier")
	get_submodule('rust-elfloader')

	ENV['CARGO_TARGET_DIR'] = nil
	run 'cargo', 'build', '--release'
end

task :ci => [:user, :deps_other] do
	case ENV['CI']
		when 'VERIFIER'
			Rake::Task["verifier"].invoke
		else
			Rake::Task["build"].invoke
	end
end

task :default => :build
