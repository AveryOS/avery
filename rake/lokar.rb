require 'strscan'

module Lokar
	if defined? Tilt
		class Template < Tilt::Template
			def prepare
				@proc = Lokar.compile(data, eval_file)
			end

			def evaluate(scope, locals, &block)
				scope.instance_eval(&@proc).join
			end
		end
		Tilt.register Template, 'lokar'
	end
	
	def self.render(string, filename = '<Lokar>', binding = nil)
		eval("__output__ = []#{parse(string, filename).join}; __output__", binding, filename).join
	end
	
	def self.compile(string, filename = '<Lokar>', binding = nil)
		eval "Proc.new do __output__ = []#{parse(string, filename).join}; __output__ end", binding, filename
	end
	
	def self.parse(string, filename)
		scanner = StringScanner.new(string)
		prev_text = false
		prev_output = true
		output = []
		line = 1
		flushed = 1
		
		while true
			match = scanner.scan_until(/(?=<\?r(?:[ \t]|$)|\#?\#{|^[ \t]*%|(?:\r\n?|\n))/m)
			if match
				# Add the text before the match
				unless prev_text
					if prev_output
						output << "<<"
					else
						output << ";__output__<<"
						prev_output = true
					end
					prev_text = true
				end
				output << match.inspect
				
				case # Find out what of the regular expression matched
					when match = scanner.scan(/\r\n?|\n/) # Parse newlines
						unless prev_text
							if prev_output
								output << "<<"
							else
								output << ";__output__<<"
								prev_output = true
							end
							prev_text = true
						end
						output << match.inspect
						line += 1
						
					when scanner.match?(/</) # Parse <?r?> tags
						scanner.pos += 3
						result = scanner.scan_until(/(?=\?>)/m)
						raise "#{filename}:#{line}: Unterminated <\?r ?> tag" unless result
						
						output << ("\n" * (line - flushed)) << ";" << result
						flushed = line
						prev_text = false
						prev_output = false
						
						scanner.pos += 2
					
					when scanner.skip(/\#/) # Parse #{ } tags
						if scanner.skip(/\#/)
							unless prev_text
								if prev_output
									output << "<<"
								else
									output << ";__output__<<"
									prev_output = true
								end
								prev_text = true
							end
							output << '#'.inspect
						else
							index = 1
							scanner.pos += 1
							
							if prev_output
								output << "<<" << ("\n" * (line - flushed)) << "("
							else
								output << ("\n" * (line - flushed)) << ";__output__<<("
								prev_output = true
							end
							flushed = line
							prev_text = false
							
							while true
								result = scanner.scan_until(/(?=}|{)/m)
								raise "#{filename}:#{line}: Unterminated \#\{ } tag" unless result
								output << result
								case
									when scanner.skip(/{/)
										index += 1
										output << '{'
										
									when scanner.skip(/}/)
										index -= 1
										break if index == 0
										output << '}'
								end
							end
							
							output << ")"
						end
					
					else # Parse %, %% and %= lines
						result = scanner.scan(/[ \t]*%/)
						if scanner.skip(/%/)
							unless prev_text
								if prev_output
									output << "<<"
								else
									output << ";__output__<<"
									prev_output = true
								end
								prev_text = true
							end
							output << result.inspect
						elsif scanner.skip(/=/)
							if prev_output
								output << "<<" << ("\n" * (line - flushed)) << "("
							else
								output << ("\n" * (line - flushed)) << ";__output__<<("
								prev_output = true
							end
							flushed = line
							prev_text = false
							output << scanner.scan_until(/(?=\r\n|\n|\Z)/) << ")"
						else
							output << ("\n" * (line - flushed)) << ";" << scanner.scan_until(/\r\n|\n|\Z/)
							flushed = line
							prev_text = false
							prev_output = false
						end
				end
			else # End of file
				unless scanner.eos?
					# Add the pending text
					unless prev_text
						if prev_output
							output << "<<"
						else
							output << ";__output__<<"
							prev_output = true
						end
						prev_text = true
					end
					output << scanner.rest.inspect
				end
				break
			end
		end
		
		output
	end
end