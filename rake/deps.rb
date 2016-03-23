
build_type = :build

# Build a unix like package at src
build_unix_pkg = proc do |src, opts, &proc|
	if build_type == :clean
		FileUtils.rm_rf(["meta/built", "meta/configured", "build", "install"])
		FileUtils.rm_rf(["#{src}/target"]) if opts[:cargo]
	end

	next if build_type != :build

	old_env = ENV.to_h
	ENV.replace(CLEANENV.merge(opts[:env] || {}))

	mkdirs("install")
	prefix = File.realpath("install");

	build_dir = opts[:intree] ? src : "build"

	mkdirs(build_dir)
	mkdirs("meta")

	unless File.exists?("meta/configured")
		Dir.chdir(build_dir) do
			old_unix = UNIX_EMU[0]
			UNIX_EMU[0] = opts[:unix]
			proc.call(File.join("..", src), prefix)
			UNIX_EMU[0] = old_unix
		end
		run 'touch', "meta/configured"
	end if proc

	built = File.exists?("meta/built")
	unless built
		bin_path = "install"

		Dir.chdir(build_dir) do
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

		run 'touch', "meta/built"
	end

	ENV.replace(old_env)
	built
end

travis_exit = proc do |built|
	if ENV['TRAVIS'] && !built
		puts "Exiting so Travis can cache the result"
		exit
	end
end

# Build a unix like package from url
build_from_url = proc do |url, name, ver, opts = {}, &proc|
	src = "#{name}-#{ver}"
	ext = opts[:ext] || "bz2"
	path = opts[:path] || name

	mkdirs(path)
	Dir.chdir(path) do
		if build_type == :clean
			FileUtils.rm_rf(src)
		else
			if !File.exists?(src)
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
		end
		travis_exit.(build_unix_pkg.(src, opts, &proc))
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
			puts "Cleaning #{name}"
			old = build_type
			build_type = :clean
			build_unix_pkg.("src", opts, &proc)
			build_type = old
		end
		build_unix_pkg.("src", opts, &proc)
	end
end

# Build a unix like package from a git submodule
build_submodule = proc do |name, opts = {}, &proc|
	mkdirs(name)
	Dir.chdir(name) do
		mkdirs("meta")
		revision = File.read("meta/revision").strip if File.exists?("meta/revision")
		current = Dir.chdir("src") { `git rev-parse --verify HEAD`.strip }
		if revision && revision != current
				puts "Cleaning #{name}... new revision #{current}"
				old = build_type
				build_type = :clean
				build_unix_pkg.("src", opts, &proc)
				build_type = old
				FileUtils.rm_rf(["meta/revision"])
		end
		built = build_unix_pkg.("src", opts, &proc)
		open("meta/revision", 'w') { |f| f.puts current } if revision != current
		travis_exit.(built)
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

EXTERNAL_BUILDS = proc do |type, real, extra|
	build_type = type

	raise "Cannot build non-UNIX dependencies with MSYS2 shell, use the MinGW shell and run `rake deps`" if ON_WINDOWS && !ON_WINDOWS_MINGW
	raise "Ninja is required on Windows" if ON_WINDOWS_MINGW && !NINJA

	#run *%w{git submodule init}
	#run *%w{git submodule update}

	Dir.chdir('vendor/') do
		build_from_url.("ftp://ftp.gnu.org/gnu/binutils/", "binutils", "2.26", {unix: true, path: 'elf-binutils'}) do |src, prefix|
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

		build_submodule.("llvm", {ninja: true}) do |src, prefix|
			#-DLLVM_ENABLE_ASSERTIONS=On  crashes on GCC 5.x + Release on Windows
			#-DCMAKE_BUILD_TYPE=RelWithDebInfo
			opts = %W{-DLLVM_TARGETS_TO_BUILD=X86 -DLLVM_EXTERNAL_CLANG_SOURCE_DIR=#{hostpath(File.join(src, '../clang'))} -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=On -DCMAKE_INSTALL_PREFIX=#{prefix}}
			opts += ['-G',  'Ninja', '-DLLVM_PARALLEL_LINK_JOBS=1'] if NINJA
			opts += %w{-DCMAKE_CXX_COMPILER=g++ -DCMAKE_C_COMPILER=gcc} if ON_WINDOWS_MINGW
			run "cmake", src, *opts
		end

		build_from_url.("ftp://ftp.gnu.org/gnu/libiconv/", "libiconv", "1.14", {unix: true, ext: "gz"}) do |src, prefix|
			run File.join(src, 'configure'), "--prefix=#{prefix}"
		end if nil #RbConfig::CONFIG['host_os'] == 'msys'

		# autotools for picky newlib

		build_from_url.("ftp://ftp.gnu.org/gnu/autoconf/", "autoconf", "2.68", {unix: true, ext: "gz"}) do |src, prefix|
			update_cfg.(src)
			update_cfg.(File.join(src, 'build-aux'))
			run File.join(src, 'configure'), "--prefix=#{prefix}"
		end

		build_from_url.("ftp://ftp.gnu.org/gnu/automake/", "automake", "1.12", {unix: true, ext: "gz"}) do |src, prefix|
			update_cfg.(src)
			update_cfg.(File.join(src, 'lib'))
			run File.join(src, 'configure'), "--prefix=#{prefix}"
		end

		build_submodule.("binutils", {unix: true}) do |src, prefix|
			run File.join(src, 'configure'), "--prefix=#{prefix}", *%w{--target=x86_64-pc-avery --with-sysroot --disable-nls --disable-werror --disable-gdb --disable-sim --disable-readline --disable-libdecnumber}
		end # binutils is buggy with mingw-w64

		build_rt = proc do |target, s, binutils, flags|
			next if !real
			next if Dir.exists?("compiler-rt/install-#{target}")
			mkdirs("compiler-rt/build-#{target}")
			Dir.chdir("compiler-rt/build-#{target}") do
				src = '../src'
				prefix = hostpath("../install-#{target}")
				opts = ["-DLLVM_CONFIG_PATH=#{File.join(src, "../../llvm/install/bin/llvm-config")}",
					"-DFREESTANDING=On",
					"-DCMAKE_SYSTEM_NAME=Generic",
					"-DCMAKE_SIZEOF_VOID_P=#{s}",
					"-DCMAKE_SYSROOT=#{hostpath("fake-sysroot")}",
					"-DCMAKE_ASM_COMPILER=clang",
					"-DCMAKE_ASM_FLAGS=--target=#{target} -B #{hostpath("../../#{binutils}")}",
					"-DCMAKE_AR=#{which "x86_64-elf-ar"}",
					"-DCMAKE_C_COMPILER=clang",
					"-DCMAKE_CXX_COMPILER=clang++",
					"-DCMAKE_C_COMPILER_TARGET=#{target}",
					"-DCMAKE_CXX_COMPILER_TARGET=#{target}",
					"-DCMAKE_STAGING_PREFIX=#{prefix}",
					"-DCMAKE_INSTALL_PREFIX=#{prefix}",
					"-DCMAKE_C_FLAGS=-ffreestanding -O2 -nostdlib #{flags} -B #{hostpath("../../#{binutils}/ld#{EXE_POST}")}",
					"-DCMAKE_CXX_FLAGS=-ffreestanding -O2 -nostdlib #{flags} -B #{hostpath("../../#{binutils}/ld#{EXE_POST}")}",
					"-DCOMPILER_RT_BUILD_SANITIZERS=Off",
					"-DCOMPILER_RT_DEFAULT_TARGET_TRIPLE=#{target}"]
				opts += ['-G',  'MSYS Makefiles'] if ON_WINDOWS_MINGW
				run "cmake", '--debug-trycompile', src, *opts
				ENV['VERBOSE'] = '1'
				run "cmake", '--build', '.', '--target', 'install'
			end
		end

		build_rt.("x86_64-pc-avery", 8, "binutils/install/x86_64-pc-avery/bin", "-fPIC")
		build_rt.("x86_64-unknown-unknown-elf", 8, "elf-binutils/install/x86_64-elf/bin", "") # Builds i386 too
		#build_rt.("i386-generic-generic", 4)

		env = {'CFLAGS' => '-fPIC'}
		build_submodule.("newlib", {env: env}) do |src, prefix|
			Dir.chdir(File.join(src, "newlib/libc/sys")) do
				run "autoconf"
				Dir.chdir("avery") do
					run "autoreconf"
				end
			end
			# -ccc-gcc-name x86_64-pc-avery-gcc
			run File.join(src, 'configure'), "--prefix=#{prefix}", "--target=x86_64-pc-avery", 'CC_FOR_TARGET=clang -fno-integrated-as -ffreestanding --target=x86_64-pc-avery'
		end

		# C++ isn't working yet
		#run "rm", "llvm/install/bin/clang++#{EXE_POST}"

		if real
			run "rm", "-rf", "avery-sysroot"
			mkdirs('avery-sysroot')
			run 'cp', '-r', 'newlib/install/x86_64-pc-avery/.', "avery-sysroot"
		end

		# CMAKE_STAGING_PREFIX, CMAKE_INSTALL_PREFIX

		build_from_git.("libcxx", "http://llvm.org/git/libcxx.git") do |src, prefix| # -nodefaultlibs
			opts = ["-DLLVM_CONFIG_PATH=#{File.join(src, "../../llvm/install/bin/llvm-config")}", "-DCMAKE_TOOLCHAIN_FILE=../../toolchain.txt", "-DCMAKE_STAGING_PREFIX=#{prefix}"]
			opts += ['-G',  'MSYS Makefiles'] if ON_WINDOWS_MINGW
			run "cmake", src, *opts
		end if nil

		# place compiler-rt in lib/rustlib/x86_64-pc-avery/lib - rustc links to it // clang links to it instead
		run 'cp', 'compiler-rt/install-x86_64-pc-avery/lib/generic/libclang_rt.builtins-x86_64.a', "avery-sysroot/lib/libcompiler_rt.a" if real

		# clang is not the host compiler, force use of gcc
		env = {'CC' => 'gcc', 'CXX' => 'g++'}
		build_submodule.("rust", {env: env}) do |src, prefix|
			run File.join(src, 'configure'), "--prefix=#{prefix}", "--llvm-root=#{File.join(src, "../../llvm/build")}", "--disable-docs", "--target=x86_64-pc-avery", "--disable-jemalloc"
		end

		run 'cp', '-r', 'rust/install/lib/rustlib/x86_64-pc-avery', "avery-sysroot/lib/rustlib" if real

		env = if which 'brew'
			prefix = `brew --prefix openssl`.strip
			{
				'OPENSSL_INCLUDE_DIR' => "#{prefix}/include",
				'OPENSSL_LIB_DIR' => "#{prefix}/lib"
			}
		else
			{}
		end
		env['CARGO_HOME'] = path('build/cargo/home')

		build_submodule.("cargo", {intree: true, env: env}) do |src, prefix|
			Dir.chdir(src) do
				run *%w{git submodule update --init}
			end
			run File.join(src, 'configure'), "--prefix=#{prefix}", "--local-rust-root=#{path("vendor/rust/install")}"
		end

		env = {'LIBCLANG_PATH' => path("vendor/llvm/install/#{ON_WINDOWS ? 'bin' : 'lib'}")}
		build_from_git.("bindgen", "https://github.com/crabtw/rust-bindgen.git", {cargo: true, env: env}) if extra
	end
end
