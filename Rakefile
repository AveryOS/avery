require 'fileutils'
require_relative 'rake/build'
require_relative 'rake/lokar'

CURRENT_DIR = Rake.original_dir
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

append_path(File.expand_path('../build/cargo/home/bin', __FILE__))
append_path(File.expand_path('../vendor/cmake/install/bin', __FILE__))
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

ENV['CARGO_HOME'] = path('build/cargo/home')
ENV['CARGO_TARGET_DIR'] = File.expand_path('../build/cargo/target', __FILE__)
ENV['RUST_TARGET_PATH'] = File.expand_path('../targets', __FILE__)
ENV['RUST_BACKTRACE'] = '1'
UNIX_EMU = [false]

NINJA = which('ninja')

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
raise "Ninja is required on Windows" if ON_WINDOWS_MINGW && !NINJA

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

RELEASE_BUILD = ENV['AVERY_BUILD'] == 'RELEASE' ? true : false
CARGO_BUILD = RELEASE_BUILD ? 'release' : 'debug'

RUSTFLAGS = ['--sysroot', hostpath('build/sysroot')] + %w{-g -Z no-landing-pads -C target-feature=-mmx,-sse,-sse2}
ENV['RUSTFLAGS_HOST'] = "-L #{hostpath("vendor/llvm/install/#{ON_WINDOWS ? "bin" : "lib"}")}" # Workaround bug with LLVM linking

def cargo(path, target, cargoflags = [], flags = [])
	cargoflags += ['--target', target]
	if target == 'x86_64-pc-avery'
		ENV['RUSTFLAGS'] = nil
	else
		ENV['RUSTFLAGS'] = RUSTFLAGS.join(" ") # Cargo uses this. How to pass spaces here?
	end
	run 'cargo', 'rustc', *(RELEASE_BUILD ? ['--release'] : []), '--manifest-path', File.join(path, 'Cargo.toml'), *cargoflags, '--', *flags
end

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

		cargo 'kernel', 'x86_64-avery-kernel', [], flags

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

		compiler_rt = 'vendor/compiler-rt/x86_64-unknown-unknown-elf/install/lib/generic/libclang_rt.builtins-x86_64.a'

		# Finally link#compiler_rt
		run 'x86_64-elf-ld', '-z', 'max-page-size=0x1000',
			'-T', build.output(File.join(gen_folder, linker_script)),
			*objects, compiler_rt,
			'-o', kernel_binary
	end
end

task :deps => [:user, :deps_other] do
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

	cargo 'kernel/arch/x64/multiboot', 'x86_32-avery-kernel', [], (%w{-C lto --emit=obj=build/bootstrap-32.o})

	compiler_rt = 'vendor/compiler-rt/x86_64-unknown-unknown-elf/install/lib/generic/libclang_rt.builtins-i386.a'

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

task :deps_other do
	EXTERNAL_BUILDS.(:build, true, false)
end

task :fmt do
	run 'cargo', 'install', 'rustfmt' unless which 'rustfmt'
	format = proc { run 'cargo', 'fmt', '--', '--config-path', File.expand_path('.') }
	Dir.chdir('kernel') { format.() }
	Dir.chdir('kernel/arch/x64/multiboot') { format.() }
end

task :user => :deps_other do
	ENV['RUSTFLAGS'] = nil

	cargo 'user', 'x86_64-pc-avery'
	cargo 'user/hello', 'x86_64-pc-avery'

	mkdirs("build/user")
	FileUtils.cp "build/cargo/target/x86_64-pc-avery/#{CARGO_BUILD}/hello", "build/user"

	build = Build.new('build', 'info.yml')
	build.run do
		build.process  "build/user/hello.o", "build/user/hello" do |o, i|
			run 'x86_64-elf-objcopy', '-I', 'binary', '-B', 'i386', '-O', 'elf64-x86-64', i, o
		end
	end
end

UPSTREAMS = {
	'vendor/llvm/clang' => 'http://llvm.org/git/clang.git',
	'vendor/llvm/src' => 'http://llvm.org/git/llvm.git',
	'vendor/compiler-rt/src' => 'http://llvm.org/git/compiler-rt.git',
	'vendor/rust/src/src/liblibc' => 'https://github.com/rust-lang/libc.git',
	'vendor/rust/src' => 'https://github.com/rust-lang/rust.git',
	'vendor/cargo/src' => 'https://github.com/rust-lang/cargo.git',
}

task :upstreams do
	UPSTREAMS.each do |(path, url)|
		Dir.chdir(path) do
			git_remote = `git remote get-url upstream`.strip
			if git_remote == ''
				run *%W{git remote add upstream #{url}}
			elsif git_remote != url
				raise "Git remote mismatch for #{path}. Local is #{git_remote}. Required is #{url}"
			end
		end
	end
end

task :rebase => :upstreams do
	Dir.chdir(CURRENT_DIR)
	path = Pathname.new(CURRENT_DIR).relative_path_from(Pathname.new(AVERY_DIR)).to_s
	puts "Rebasing in #{path}.."
	remote_branch = {
		'vendor/llvm/clang' => 'release_38',
		'vendor/llvm/src' => 'release_38',
	}[path] || "master"
	raise "No remote for path #{path}" unless UPSTREAMS[path]
	local_master = `git rev-parse master`.strip
	local_master = nil if $?.exitstatus != 0
	unless local_master
		local = `git rev-parse avery`.strip
		remote = `git rev-parse origin/avery`.strip
		if local == remote
			run *%w{git checkout -b master origin/master}
		else
			raise "master branch doesn't exist. Don't know the start of the rebase"
		end
	end
	run *%w{git fetch upstream}
	run *%w{git checkout avery}
	run_stay *%W{git rebase -i --onto upstream/#{remote_branch} master avery}
	if $? != 0
		loop do
			action = loop do
				puts "Continue (c) or abort (a)?"
				case STDIN.gets.strip
					when "c"
							break true
					when "a"
							break false
				end
			end

			if action
				puts "Continuing.."
				run_stay *%w{git rebase --continue}
				break if $? == 0
			else
				puts "Aborting.."
				run_stay *%w{git rebase --abort}
				raise "Rebase aborted"
			end
		end
	end
	run *%w{git checkout master}
	run *%W{git reset --hard upstream/#{remote_branch}}
	run *%w{git checkout avery}

	action = loop do
		puts "Push changes? y/n?"
		case STDIN.gets.strip
			when "y"
					break true
			when "n"
					break false
		end
	end
	if action
		run *%w{git push origin master}
		run *%w{git push origin avery -f}
	end
end

task :sh do
	Dir.chdir(CURRENT_DIR)
	run 'bash'
end

task :default => :build
