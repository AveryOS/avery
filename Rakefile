require 'fileutils'
require 'lokar'
require_relative 'rake/build'

ENV['PATH'] += ";#{File.expand_path('../bin', __FILE__)}" if Gem.win_platform?

QEMU = "#{'qemu/' if Gem.win_platform?}qemu-system-x86_64"
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
		build.execute AS, source.path, '-o', object_file
	end
	
	objects << object_file
end

RUSTFLAGS = %w{--opt-level 3 -C no-stack-check -C relocation-model=static -C code-model=kernel -C no-redzone -Z no-landing-pads}

def rust_base(build, prefix, flags)
	core = File.join(prefix, "crates/libcore.rlib")
	rlibc = File.join(prefix, "crates/librlibc.rlib")

	build.mkdirs(File.join(prefix, "crates/."))

	build.execute 'rustc', *RUSTFLAGS, *flags, '--crate-type=rlib', 'vendor/rust/src/libcore/lib.rs', '-o', core
	build.execute 'rustc', '-L', File.join(prefix, "crates"), *RUSTFLAGS, *flags,  '--crate-type=rlib', 'vendor/rust/src/librlibc/lib.rs', '-o', rlibc
end

def rust_crate(build, base_prefix, prefix, flags, src, src_flags)
	build.mkdirs(File.join(prefix, "."))
	build.execute 'rustc', '-C', 'lto', '-L', File.join(base_prefix, "crates"), '-L', 'build/phase', *RUSTFLAGS, *flags, src,  '--out-dir', prefix, *src_flags
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
	
	boot_files = sources.extract('src/arch/x64/boot/**/*')
	multiboot_files = sources.extract('src/arch/x64/multiboot/**/*')

	if type == :multiboot
		sources.add multiboot_files
	else
		sources.add boot_files
	end
	
	bitcodes = []
	bitcodes_bootstrap = []
	objects = ['vendor/font.o', kernel_object]

	build.run do
		rust_crate(build, "build", build.output("#{type}"), %w{--target x86_64-unknown-linux-gnu}, 'src/kernel.rs', ['--emit=obj,ir'] + (type == :multiboot ? ['--cfg', 'multiboot'] : []))
	
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
		
		#bitcode_link(build, kernel_object, build.output("#{type}/kernel_linked.bc"), bitcodes, ['-disable-red-zone', '-code-model=kernel'])

		build.process kernel_binary, *objects, kernel_linker_script do
			build.execute LD, '-z', 'max-page-size=0x1000', '-T', kernel_linker_script, *objects, '-o', kernel_binary
		end
		
		case type
			when :multiboot
				build.execute 'mcopy', '-D', 'o', '-D', 'O', '-i' ,'emu/grubdisk.img@@1M', kernel_binary, '::kernel.elf'
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
		build.mkdirs("build/phase/.")
		build.execute 'rustc', '-O', '--out-dir', "build/phase", "vendor/asm/assembly.rs"

		rust_base(build, build.output(""), %w{--target x86_64-unknown-linux-gnu})
		rust_base(build, build.output("bootstrap"), %w{--target i686-unknown-linux-gnu})

		rust_crate(build, build.output("bootstrap"), build.output("bootstrap"), %w{--target i686-unknown-linux-gnu}, 'src/arch/x64/multiboot/bootstrap.rs', ['--emit=asm,ir'])

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

		build.execute AS, asm, '-o', kernel_object_bootstrap
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

task :qemu => :build do
	Dir.chdir('emu/') do
		puts "Running QEMU..."
		Build.execute QEMU, *%w{-L qemu\Bios -hda grubdisk.img -serial file:serial.txt -d int,cpu_reset -no-reboot -s -smp 4}
	end
end

task :qemu_efi => :build_boot do
	Dir.chdir('emu/') do
		puts "Running QEMU..."
		Build.execute QEMU, *%w{-L . -bios OVMF.fd -hda fat:hda -serial file:serial.txt -d int,cpu_reset -no-reboot -s -smp 4}
	end
end

task :bochs => :build do
	
	Dir.chdir('emu/') do
		puts "Running Bochs..."
		Build.execute 'bochs\bochs', '-q', '-f', 'avery.bxrc'
	end
end

task :default => :build
