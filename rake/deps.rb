
def get_submodule(path)
	return if File.exists?(File.join(path, '.git'))
	run *%w{git submodule update --init}, *(ENV['TRAVIS'] ? ['--depth', '1'] : []), path.shellescape
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
	run *%w{git fetch upstream --no-recurse-submodules}
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

build_type = :build

def rebuild(rev_file, depends = [], version = "")
	built_rev = File.read(rev_file).strip if File.exist?(rev_file)
	digest = Digest::SHA2.new(256)
	digest << version
	depends.each do |d|
		digest << File.read("vendor/#{d}/meta/revision").strip
	end
	rev = digest.hexdigest
	if built_rev != rev
			FileUtils.rm_rf([rev_file])
			r = yield
			mkdirs(File.dirname(rev_file))
			open(rev_file, 'w') { |f| f.puts rev }
			r
	end
end

# Build a unix like package at src
build_unix_pkg = proc do |src, rev, opts, config, &gen_src|
	vendor = Pathname.new(File.join(AVERY_DIR, 'vendor'))
	pathname = Pathname.new(File.expand_path('.')).relative_path_from(vendor).to_s
	cache = File.expand_path(File.join(src, "../../cache", pathname))

	clean = proc do
		FileUtils.rm_rf(["meta", "build", "install"])
		FileUtils.rm_rf([cache]) if ENV['TRAVIS']
		FileUtils.rm_rf(["#{src}/target"]) if opts[:cargo]
	end

	if build_type == :clean
		clean.()
	end

	next if build_type != :build

	test_prefix = ENV['TRAVIS'] ? "#{cache}/" : ""
	test_revision = "#{test_prefix}meta/revision"

	built_rev = File.read(test_revision).strip if File.exists?(test_revision)
	if built_rev && built_rev != rev
			puts "Cleaning #{pathname} #{"(version #{built_rev})" if built_rev}... new version #{rev}"
			FileUtils.rm_rf([test_revision])
			if opts[:noclean] && !ENV['TRAVIS']
				FileUtils.rm_rf(["meta/built"])
			else
				clean.()
			end
	end

	if ENV['TRAVIS'] && File.exists?(test_revision)
		if pathname == 'llvm'
			# clang is hardcoded to use relative paths; symlinks won't work
			run 'cp', '-r', "#{cache}/install", File.expand_path('.')
		else
			run 'ln', '-s', "#{cache}/install", File.expand_path('install')
		end
		run 'ln', '-s', "#{cache}/meta", File.expand_path('meta')
		next
	end

	old_env = ENV.to_hash
	ENV.replace(CLEANENV.merge(opts[:env] || {}))

	mkdirs("install")
	prefix = File.realpath("install");

	build_dir = opts[:intree] ? src : "build"

	mkdirs("meta")

	unless File.exists?("meta/configured")
		puts "Configuring #{pathname} (version #{rev})..."
		gen_src.call() if gen_src
		mkdirs(build_dir)
		Dir.chdir(build_dir) do
			old_unix = UNIX_EMU[0]
			UNIX_EMU[0] = opts[:unix]
			config.call(File.join("..", src), prefix)
			UNIX_EMU[0] = old_unix
		end if config
		run 'touch', "meta/configured"
	end

	unless File.exists?("meta/built")
		puts "Building #{pathname} (version #{rev})..."
		mkdirs(build_dir)
		bin_path = "install"

		Dir.chdir(build_dir) do
			if opts[:cargo]
				run "cargo", "install", "--path=#{File.join("..", src)}", "--root=#{File.join("..", 'install')}"
			else
				if opts[:ninja] && NINJA
					p = ENV['TRAVIS'] ? ['-j1'] : []
					run "ninja", *p
					run "ninja", "install", *p
				else
					old_unix = UNIX_EMU[0]
					UNIX_EMU[0] = opts[:unix]
					b = opts[:make]
					if b
						b.()
					else
						run "make", "-j#{CORES}"
						run "make", "install"
					end
					UNIX_EMU[0] = old_unix
				end
			end
		end

		run 'touch', "meta/built"

	end

	open("meta/revision", 'w') { |f| f.puts rev }

	if ENV['TRAVIS']
		mkdirs(cache)
		run 'cp', '-r', 'install', cache
		run 'cp', '-r', 'meta', cache
	end

	ENV.replace(old_env)
end

# Build a unix like package from url
build_from_url = proc do |url, name, ver, opts = {}, &proc|
	src = "#{File.basename(name)}-#{ver}"
	ext = opts[:ext] || "bz2"
	path = opts[:path] || name

	mkdirs(path)
	Dir.chdir(path) do
		if build_type == :clean
			FileUtils.rm_rf(src)
		end
		build_unix_pkg.(src, src, opts, proc) do
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
			build_unix_pkg.("src", opts, proc)
			build_type = old
		end
		rev = Dir.chdir("src") { `git rev-parse --verify HEAD`.strip }
		build_unix_pkg.("src", rev, opts, proc)
	end
end

# Build a unix like package from a git submodule
build_submodule = proc do |name, opts = {}, &proc|
	mkdirs(name)
	Dir.chdir(name) do
		rev = `git ls-tree HEAD src`.strip.split(" ")[2]
		build_base = opts[:build] || '.'
		mkdirs(build_base)
		src_path = Pathname.new('src').relative_path_from(Pathname.new(build_base)).to_s
		Dir.chdir(build_base) do
			build_unix_pkg.(src_path, rev, opts, proc) do
				([src_path] + (opts[:submodules] || [])).each do |s|
					get_submodule(s)
				end
			end
		end
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

task :dep_cmake do
	build_from_url.("https://cmake.org/files/v3.5/", "vendor/cmake", "3.5.0", {ext: "gz"}) do |src, prefix|
		run File.join(src, 'configure'), "--prefix=#{prefix}"
	end unless `cmake --version`.include?('3')
end

task :dep_elf_binutils do
	build_from_url.("ftp://ftp.gnu.org/gnu/binutils/", "binutils", "2.26", {unix: true, path: 'vendor/elf-binutils'}) do |src, prefix|
		run File.join(src, 'configure'), "--prefix=#{prefix}", *%w{--target=x86_64-elf --with-sysroot --disable-nls --disable-werror}
	end # binutils is buggy with mingw-w64
end

task :dep_mtools do
	build_from_url.("ftp://ftp.gnu.org/gnu/mtools/", "vendor/mtools", "4.0.18", {unix: true}) do |src, prefix|
		update_cfg.(src)
		#run 'cp', '-rf', "../../libiconv/install", ".."
		Dir.chdir(src) do
			run 'patch', '-i', "../../mtools-fix.diff"
		end
		opts = []
		opts += ["LIBS=-liconv"] if Gem::Platform::local.os == 'darwin'
		run File.join(src, 'configure'), "--prefix=#{prefix}", *opts
	end # mtools doesn't build with mingw-w64
end

task :dep_llvm => :dep_cmake do
	build_submodule.("vendor/llvm", {ninja: true, noclean: true, submodules: ["clang"]}) do |src, prefix|
		#-DLLVM_ENABLE_ASSERTIONS=On  crashes on GCC 5.x + Release on Windows
		#-DCMAKE_BUILD_TYPE=RelWithDebInfo
		opts = %W{-DLLVM_TARGETS_TO_BUILD=X86 -DLLVM_EXTERNAL_CLANG_SOURCE_DIR=#{hostpath(File.join(src, '../clang'))} -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=On -DCMAKE_INSTALL_PREFIX=#{prefix}}
		opts += ['-G',  'Ninja', '-DLLVM_PARALLEL_LINK_JOBS=1'] if NINJA
		opts += %w{-DCMAKE_CXX_COMPILER=g++ -DCMAKE_C_COMPILER=gcc} if ON_WINDOWS_MINGW
		run "cmake", src, *opts
	end
end

task :dep_autoconf do
	build_from_url.("ftp://ftp.gnu.org/gnu/autoconf/", "vendor/autoconf", "2.68", {unix: true, ext: "gz"}) do |src, prefix|
		update_cfg.(src)
		update_cfg.(File.join(src, 'build-aux'))
		run File.join(src, 'configure'), "--prefix=#{prefix}"
	end
end

task :dep_automake do
	build_from_url.("ftp://ftp.gnu.org/gnu/automake/", "vendor/automake", "1.12", {unix: true, ext: "gz"}) do |src, prefix|
		update_cfg.(src)
		update_cfg.(File.join(src, 'lib'))
		run File.join(src, 'configure'), "--prefix=#{prefix}"
	end
end

task :dep_binutils do
	build_submodule.("vendor/binutils", {unix: true}) do |src, prefix|
		run File.join(src, 'configure'), "--prefix=#{prefix}", *%w{--target=x86_64-pc-avery --with-sysroot --disable-nls --disable-werror --disable-gdb --disable-sim --disable-readline --disable-libdecnumber}
	end # binutils is buggy with mingw-w64
end

task :dep_compiler_rt => [:dep_llvm, :dep_binutils, :dep_elf_binutils] do
	build_rt = proc do |target, s, binutils, flags|
		build_submodule.("vendor/compiler-rt", {build: target}) do |src, prefix|
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
			run "cmake", src, *opts
		end
	end

	build_rt.("x86_64-pc-avery", 8, "binutils/install/x86_64-pc-avery/bin", "-fPIC")
	build_rt.("x86_64-unknown-unknown-elf", 8, "elf-binutils/install/x86_64-elf/bin", "") # Builds i386 too
	#build_rt.("i386-unknown-unknown-elf", 4, "elf-binutils/install/x86_64-elf/bin", "-m32")
end

task :dep_newlib => [:dep_llvm, :dep_automake, :dep_autoconf, :dep_binutils] do
	env = {'CFLAGS' => '-fPIC'}
	build_submodule.("vendor/newlib", {env: env, noclean: true}) do |src, prefix|
		Dir.chdir(File.join(src, "newlib/libc/sys")) do
			run "autoconf"
			Dir.chdir("avery") do
				run "autoreconf"
			end
		end
		# -ccc-gcc-name x86_64-pc-avery-gcc
		run File.join(src, 'configure'), "--prefix=#{prefix}", "--target=x86_64-pc-avery", 'CC_FOR_TARGET=clang -fno-integrated-as -ffreestanding -fno-inline -fno-optimize-sibling-calls -fno-omit-frame-pointer --target=x86_64-pc-avery'
	end
end

task :avery_sysroot => [:dep_compiler_rt, :dep_newlib] do
	rebuild("vendor/avery-sysroot/version", ["compiler-rt/x86_64-pc-avery", "newlib"]) do
		run "rm", "-rf", "vendor/avery-sysroot"
		mkdirs('vendor/avery-sysroot')
		run 'cp', '-r', 'vendor/newlib/install/x86_64-pc-avery/.', "vendor/avery-sysroot"

		# place compiler-rt in lib/rustlib/x86_64-pc-avery/lib - clang links to it
		run 'cp', 'vendor/compiler-rt/x86_64-pc-avery/install/lib/generic/libclang_rt.builtins-x86_64.a', "vendor/avery-sysroot/lib/libcompiler_rt.a"
	end
end

task :dep_rust => [:dep_llvm, :avery_sysroot] do
	# clang is not the host compiler, force use of gcc
	env = {
		'CC' => CC || 'gcc',
		'CXX' => CXX || 'g++',
		'CARGO_HOME' => path('build/cargo/home'),
	}
	prefix = File.expand_path("vendor/rust/install")
	make = proc do
		run *%W{make dist -j#{CORES}}
		install = proc do |n|
				dist = Dir["build/dist/#{n}-*"][0]
				target = "extract-dist/#{n}"
				mkdirs(target)
				run 'tar', "-zxf", dist, '-C', target
				target = Dir["#{target}/*"][0]
				run "bash", "#{target}/install.sh", "--prefix=#{prefix}"
				triple = File.basename(target).split('-')[3..-1].join('-')
				dest = path("vendor/rust/install/lib/rustlib/#{triple}/lib")
				mkdirs(dest)
				# Copy shared libraries for rustc into the host lib direcory
				run 'cp', '-r', path("vendor/rust/install/bin/."), dest
				Dir["vendor/rust/install/lib/*.so"].each do |f|
					run 'cp', f, dest
				end
		end
		install.('rustc')
		install.('rust-std')
	end
	build_submodule.("vendor/rust", {env: env, noclean: true, make: make}) do |src, prefix|
		llvm_path = File.expand_path(File.join(src, "../../llvm/install"))
		run File.join(src, 'configure'), '--enable-rustbuild', "--prefix=#{prefix}", "--llvm-root=#{llvm_path}", "--disable-docs", "--disable-jemalloc"
	end
end

task :dep_cargo => :dep_rust do
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

	build_submodule.("vendor/cargo", {intree: true, env: env}) do |src, prefix|
		Dir.chdir(src) do
			run *%w{git submodule update --init}
		end
		run File.join(src, 'configure'), "--prefix=#{prefix}", "--local-rust-root=#{path("vendor/rust/install")}"
	end
end

task :dep_bindgen => :dep_llvm do
	raise "Need rustc to build bindgen" unless which('rustc')
	env = {'LIBCLANG_PATH' => path("vendor/llvm/install/#{ON_WINDOWS ? 'bin' : 'lib'}")}
	build_from_git.("vendor/bindgen", "https://github.com/crabtw/rust-bindgen.git", {cargo: true, env: env})
end

EXTERNAL_BUILDS = proc do |type, real, extra|
	travis_pause = proc do
		mins = (Time.new - START_TIME) / 60.0
		if ENV['TRAVIS'] && mins > 5
			puts "Exiting so Travis can cache the result"
			exit
		end
	end

	build_type = type

	build_from_url.("ftp://ftp.gnu.org/gnu/libiconv/", "vendor/libiconv", "1.14", {unix: true, ext: "gz"}) do |src, prefix|
		run File.join(src, 'configure'), "--prefix=#{prefix}"
	end if nil #RbConfig::CONFIG['host_os'] == 'msys'

	Rake::Task["dep_mtools"].invoke
	Rake::Task["dep_cmake"].invoke

	travis_pause.()
	Rake::Task["dep_llvm"].invoke
	travis_pause.()

	Rake::Task["avery_sysroot"].invoke

	travis_pause.()
	Rake::Task["dep_rust"].invoke
	travis_pause.()

	# Copy compiler libraries into the library search path
	Dir["vendor/rust/install/lib/rustlib/*"].each do |dir|
		next if File.basename(dir) == 'x86_64-pc-avery'
		target = "#{dir}/lib"
		next if Dir["#{target}/libstd-*"].empty?
		next unless Dir["#{target}/rustc-*"].empty?
		run 'cp', '-r', "vendor/rust/install/bin/.", target
		Dir["vendor/rust/install/lib/*.so"].each do |f|
			run 'cp', f, target
		end
	end

	# CMAKE_STAGING_PREFIX, CMAKE_INSTALL_PREFIX

	build_from_git.("vendor/libcxx", "http://llvm.org/git/libcxx.git") do |src, prefix| # -nodefaultlibs
		opts = ["-DLLVM_CONFIG_PATH=#{File.join(src, "../../llvm/install/bin/llvm-config")}", "-DCMAKE_TOOLCHAIN_FILE=../../toolchain.txt", "-DCMAKE_STAGING_PREFIX=#{prefix}"]
		opts += ['-G',  'MSYS Makefiles'] if ON_WINDOWS_MINGW
		run "cmake", src, *opts
	end if nil

	Rake::Task["dep_cargo"].invoke

	# We need rust sources to build sysroots
	get_submodule('vendor/rust/src')
	Dir.chdir('vendor/rust/src/src') do
		get_submodule('liblibc')
	end
	# We need the ELF loader for the kernel
	get_submodule('verifier/rust-elfloader')

	Rake::Task["std"].invoke

	# Reset cargo target dir if rust changes
	rebuild("build/cargo/version", ["rust"]) do
		run "rm", "-rf", "build/cargo/target"
	end
end

task :re_newlib do
	run "rm", "-rf", "vendor/newlib/meta/built"
	run "rm", "-rf", "vendor/avery-sysroot/version"
	run "rm", "-rf", "build/cargo/target/x86_64-pc-avery"
end
