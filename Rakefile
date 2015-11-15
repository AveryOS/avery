require 'fileutils'
require_relative 'rake/build'
require_relative 'rake/lokar'

ENV['RUST_TARGET_PATH'] = File.expand_path('../targets', __FILE__)
ENV['RUST_BACKTRACE'] = '1'

PREFIX = File.expand_path('../vendor/install/bin', __FILE__)

def mkdirs(target)
	FileUtils.makedirs(target)
end

def run(*cmd)
	puts cmd.join(" ")
	system([cmd.first, cmd.first], *cmd[1..-1])
	raise "Command #{cmd.join(" ")} failed with error code #{$?}" if $? != 0
end

if Gem.win_platform?
	ENV['PATH'] += ";#{PREFIX}"
else
	ENV['PATH'] += ":#{PREFIX}"
end

QEMU = "#{'qemu/' if Gem.win_platform?}qemu-system-x86_64"
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

RUSTFLAGS = ['-C', "ar=#{File.join(PREFIX, AR)}", '--sysroot', File.expand_path('../build/sysroot', __FILE__)] +
	%w{-C opt-level=1 -C debuginfo=1 -Z no-landing-pads}

def rust_base(build, prefix, flags)
	crates = File.join(prefix, "crates")

	mkdirs(crates)

	run 'rustc', *RUSTFLAGS, *flags, 'vendor/rust/src/libcore/lib.rs', '--out-dir', crates
	run 'rustc', '-L', File.join(prefix, "crates"), *RUSTFLAGS, *flags, '--crate-type', 'rlib', '--crate-name', 'rlibc', 'vendor/rlibc/src/lib.rs', '--out-dir', crates
end

def rust_crate(build, base_prefix, prefix, flags, src, src_flags)
	mkdirs(prefix)
	run 'rustc', '-C', 'target-feature=-mmx,-sse,-sse2', '-C', 'lto', '-L', File.join(base_prefix, "crates"), '-L', 'build/phase', *RUSTFLAGS, *flags, src,  '--out-dir', prefix, *src_flags
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
		rust_crate(build, "build", build.output("#{type}"), %w{--target x86_64-avery-kernel}, 'src/kernel.rs', ['--emit=obj,llvm-ir'] + (type == :multiboot ? ['--cfg', 'multiboot'] : []))

		interrupts_asm = 'src/arch/x64/interrupts.s'
		interrupts_asm_out = build.output File.join("gen", interrupts_asm)

		sources.extract(interrupts_asm)

		build.process interrupts_asm_out, interrupts_asm do |o, i|
			preprocess(interrupts_asm, interrupts_asm_out, binding)
		end

		sources.add build.package(interrupts_asm_out)

		sources.each do |source|
			case source.ext.downcase
				when '.s'
					assemble(build, source, objects)
			end
		end

		puts "Linking..."

		objects << kernel_object_bootstrap if type == :multiboot

		kernel_linker_script = build.output "#{type}/kernel.ld"

		build.process kernel_linker_script, 'src/arch/x64/kernel.ld' do |o, i|
			multiboot = type == :multiboot
			preprocess(i, kernel_linker_script, binding)
		end

		objects.each do |obj|
			run 'x86_64-elf-objcopy', '--set-section-flags', '.debug*=alloc,contents,load,readonly,data,debug', obj
		end

		build.process kernel_binary, *objects, kernel_linker_script do
			run LD, '-z', 'max-page-size=0x1000', '-T', kernel_linker_script, *objects, '-o', kernel_binary
		end

		case type
			when :multiboot
				run 'mcopy', '-D', 'o', '-D', 'O', '-i' ,'emu/grubdisk.img@@1M', kernel_binary, '::kernel.elf'
			when :boot
				FileUtils.cp kernel_binary, "emu/hda/efi/boot"
		end
	end
end

task :user do
	build_user.call
end

task :base do
	build = Build.new('build', 'info.yml')
	build.run do
		mkdirs("build/phase")
		run 'rustc', '-O', '--out-dir', "build/phase", "vendor/asm/assembly.rs"

		rust_base(build, build.output(""), %w{--target x86_64-avery-kernel})

		run 'rustc', '-L', 'build/crates', *RUSTFLAGS, '--target', 'x86_64-avery-kernel', 'src/std/std.rs', '--out-dir', build.output("crates")

		rust_base(build, build.output("bootstrap"), %w{--target x86_32-avery-kernel})

		rust_crate(build, build.output("bootstrap"), build.output("bootstrap"), %w{--target x86_32-avery-kernel}, 'src/arch/x64/multiboot/bootstrap.rs', ['--emit=asm,llvm-ir']) #, '-C', 'llvm-args=-x86-asm-syntax=intel'

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
		run 'x86_64-elf-objcopy', '--strip-debug', '-G', 'setup_long_mode', kernel_object_bootstrap
	end
end

task :build do
	type = :multiboot
	build_kernel.call
end

task :build_boot do
	type = :boot
	build_kernel.call
end

task :vmware do
	Dir.chdir('emu/') do
		run *%w{qemu\qemu-img convert -O vmdk grubdisk.img avery.vmdk}
	end
end

task :qemu => :build do
	Dir.chdir('emu/') do
		puts "Running QEMU..."
		FileUtils.rm("serial.txt") if File.exists?("serial.txt")
		FileUtils.rm("int.log") if File.exists?("int.log")
		 # -d ,cpu_reset
		run QEMU, *%w{-L qemu\Bios -hda grubdisk.img -serial file:serial.txt -d int -D int.log -no-reboot -s -smp 4}
	end
end

task :qemu_efi => :build_boot do
	Dir.chdir('emu/') do
		puts "Running QEMU..."
		FileUtils.rm("serial.txt") if File.exists?("serial.txt")
		FileUtils.rm("int.log") if File.exists?("int.log")
		run QEMU, *%w{-L . -bios OVMF.fd -hda fat:hda -serial file:serial.txt -d int,cpu_reset -no-reboot -s -smp 4}
	end
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

task :vendor do
	download = proc do |url, name|
	end

	build = proc do |url, name, ver, ext = "bz2", &proc|
		src = "#{name}-#{ver}"

		mkdirs(name)
		Dir.chdir(name) do
			mkdirs("install")
			prefix = File.realpath("install");

			unless File.exists?("built")
				tar = "#{src}.tar.#{ext}"
				unless File.exists?(tar)
					run 'curl', '-O', "#{url}#{tar}"
				end

				run 'rm', '-rf', src

				uncompress = case ext
					when "bz2"
						"j"
					when "xz"
						"J"
					when "gz"
						"z"
				end

				run 'tar', "#{uncompress}xf", tar

				run 'rm', '-rf', "build"
				mkdirs("build")
				Dir.chdir("build") do
					proc.call(File.join("..", src), prefix)
					run "make", "-j4"
					run "make", "install"
				end
				run 'rm', '-rf', "build"
				#run 'rm', '-rf', src
				run 'touch', "built"
			end

			run 'cp', '-rf', 'install', ".."
		end
	end

	Dir.chdir('vendor/') do
		run 'rm', '-rf', "install" unless Gem.win_platform?
		mkdirs("install")

		build.("ftp://ftp.gnu.org/gnu/binutils/", "binutils", "2.25") do |src, prefix|
			run File.join(src, 'configure'), "--prefix=#{prefix}", *%w{--target=x86_64-elf --with-sysroot --disable-nls --disable-werror}
		end unless Gem.win_platform?

		build.("ftp://ftp.gnu.org/gnu/libiconv/", "libiconv", "1.14", "gz") do |src, prefix|
			run File.join(src, 'configure'), "--prefix=#{prefix}"
		end if nil #RbConfig::CONFIG['host_os'] == 'msys'

		build.("ftp://ftp.gnu.org/gnu/mtools/", "mtools", "4.0.18") do |src, prefix|
			#run 'cp', '-rf', "../../libiconv/install", ".."
			Dir.chdir(src) do
				run 'patch', '-i', "../../mtools-fix.diff"
				run 'patch', '-i', "../../mtools-fix2.diff"
			end
			run File.join(src, 'configure'), "--prefix=#{prefix}", "LIBS=-liconv"
		end unless Gem.win_platform?

		build.("ftp://ftp.gnu.org/gnu/grub/", "grub", "2.00", "xz") do |src, prefix|
			run 'cp', '-rf', "../../binutils/install", ".."
			run File.join(src, 'configure'), "--prefix=#{prefix}", '--target=x86_64-elf', '--disable-nls'
		end if nil

		unless Dir.exists?("rust")
			run "git", "clone" , "https://github.com/rust-lang/rust.git"
		end

		unless Dir.exists?("rlibc")
			run "git", "clone" , "https://github.com/rust-lang/rlibc.git"
		end
	end
end

task :update do
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
