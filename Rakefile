require 'fileutils'
require_relative 'rake/build'
require_relative 'rake/lokar'

AVERY_DIR = File.expand_path('../', __FILE__)
Dir.chdir(AVERY_DIR)

def path(p)
  File.expand_path(File.join('..', p), __FILE__)
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

append_path(File.expand_path('../vendor/elf-binutils/install/bin', __FILE__))
append_path(File.expand_path('../vendor/mtools/install/bin', __FILE__))
append_path(File.expand_path('../vendor/llvm/install/bin', __FILE__))
append_path(File.expand_path('../vendor/binutils/install/bin', __FILE__))
append_path(File.expand_path('../vendor/cargo/install/bin', __FILE__))
append_path(File.expand_path("../vendor/rust/install/bin", __FILE__))
append_path(File.expand_path("../vendor/autoconf/install/bin", __FILE__))
append_path(File.expand_path("../vendor/automake/install/bin", __FILE__))

sos = path("vendor/llvm/install/lib") + ":" + path("vendor/rust/install/lib")
ENV['DYLD_LIBRARY_PATH'] = sos
ENV['LD_LIBRARY_PATH'] = sos

# rustc build needs LLVM in PATH on Windows
CLEANENV = ENV.to_h

ENV['CARGO_HOME'] = path('build/cargo/home')
ENV['CARGO_TARGET_DIR'] = File.expand_path('../build/cargo/target', __FILE__)
ENV['RUST_TARGET_PATH'] = File.expand_path('../targets', __FILE__)
ENV['RUST_BACKTRACE'] = '1'
UNIX_EMU = [false]

def hostpath(p)
	if ENV['MSYSTEM']
		File.expand_path(p).sub('/', '').sub('/', ':/')
	else
		File.expand_path(p)
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

RELEASE_BUILD = false
CARGO_BUILD = RELEASE_BUILD ? 'release' : 'debug'

def cargo(path, cargoflags, flags = [])
	run 'cargo', 'rustc', *(RELEASE_BUILD ? ['--release'] : []), '--manifest-path', File.join(path, 'Cargo.toml'), *cargoflags, '--', *flags
end

# We need to pass along sysroot here so rustc won't try to use host crates. The sysroot folder doesn't need to exist.
RUSTFLAGS = ['--sysroot', hostpath('build/sysroot')] + %w{-g -Z no-landing-pads -C target-feature=-mmx,-sse,-sse2}
ENV['RUSTFLAGS'] = RUSTFLAGS.join(" ") # Cargo uses this. How to pass spaces here?
ENV['RUSTFLAGS_HOST'] = "-L #{hostpath("vendor/llvm/install//#{ON_WINDOWS ? "bin" : "lib"}")}" # Workaround bug with LLVM linking
kernel_object_bootstrap = "build/bootstrap.o"

type = :multiboot
build_kernel = proc do
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
		
		cargo 'kernel', %w{--target x86_64-avery-kernel}, flags
	
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

		compiler_rt = 'vendor/compiler-rt/install-x86_64-generic-generic/lib/generic/libclang_rt.builtins-x86_64.a'

		# Finally link#compiler_rt
		run 'x86_64-elf-ld', '-z', 'max-page-size=0x1000', '-T', build.output(File.join(gen_folder, linker_script)), *objects, compiler_rt, '-o', kernel_binary

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
	build_core = proc do |target|
		cargo 'vendor/rust/src/src/libcore', %W{--features disable_float --target #{target}}
		mkdirs("build/sysroot/lib/rustlib/#{target}/lib")
		FileUtils.cp "build/cargo/target/#{target}/#{CARGO_BUILD}/libcore.rlib", "build/sysroot/lib/rustlib/#{target}/lib"
	end
	build_core.('x86_32-avery-kernel')
	build_core.('x86_64-avery-kernel')

	cargo 'kernel/arch/x64/multiboot', %w{--target x86_32-avery-kernel}, (%w{-C lto --emit=obj=build/bootstrap-32.o} + (RELEASE_BUILD ? [] : []))
	
	compiler_rt = 'vendor/compiler-rt/install-x86_64-generic-generic/lib/generic/libclang_rt.builtins-i386.a'

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

task :qemu => :build do
	run_qemu(false)
end

task :qemu_efi => :build_efi do
	run_qemu(true)
end

task :bochsdbg => :build do
	Dir.chdir('emu/') do
		puts "Running Bochs..."
		run 'bochs/bochsdbg', '-q', '-f', 'avery.bxrc'
	end
end

task :bochs4 => :build do
	Dir.chdir('emu/') do
		puts "Running Bochs..."
		run 'bochs4/bochs', '-q', '-f', 'avery4.bxrc'
	end
end

task :bochs => :build do

	Dir.chdir('emu/') do
		puts "Running Bochs..."
		run 'bochs/bochs', '-q', '-f', 'avery.bxrc'
	end
end

require_relative 'rake/deps'

CORES = 4

task :deps_other do
	EXTERNAL_BUILDS.(:build, true, false)
end

task :extra do
	EXTERNAL_BUILDS.(:build, true, true)
end

task :update do
	EXTERNAL_BUILDS.(:update, false, true)
end

task :update_all => :update do
	#checkout_git.(".", "https://github.com/AveryOS/avery.git")
end

task :clean do
	EXTERNAL_BUILDS.(:clean, false, true)
end

task :user => :extra do
	ENV['RUSTFLAGS'] = nil
	
	cargo 'user', %w{--target x86_64-pc-avery}
	cargo 'user/hello', %w{--target x86_64-pc-avery}
end

task :sh do
	run 'bash'
end

task :default => :build
