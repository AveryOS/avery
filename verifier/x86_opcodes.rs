pub fn decode(mut bytes: &[u8]) -> Option<usize> {
	Some({
		let byte = if let Some((&byte, rest)) = bytes.split_first() {
			bytes = rest;
			b
		} else {
			return None;
		};
		match byte {
			0x0 => {
				0x34 /* add */
			}
			0x1 => {
				0x40 /* add */
			}
			0x2 => {
				0x46 /* add */
			}
			0x3 => {
				0x4b /* add */
			}
			0x4 => {
				0x7 /* add */
			}
			0x5 => {
				0xa /* add */
			}
			0x8 => {
				0x34 /* or */
			}
			0x9 => {
				0x40 /* or */
			}
			0xa => {
				0x46 /* or */
			}
			0xb => {
				0x4b /* or */
			}
			0xc => {
				0x7 /* or */
			}
			0xd => {
				0xa /* or */
			}
			0xf => {
				let byte = if let Some((&byte, rest)) = bytes.split_first() {
					bytes = rest;
					b
				} else {
					return None;
				};
				match byte {
					0xb => {
						0x0 /* ud2 */
					}
					0x10 => {
						if prefix(0x66) {
							return 0x53; /* movupd */
						}
						if prefix(0xf2) {
							return 0x52; /* movsd */
						}
						0x53 /* movups */
					}
					0x11 => {
						if prefix(0x66) {
							return 0x44; /* movupd */
						}
						if prefix(0xf2) {
							return 0x43; /* movsd */
						}
						0x44 /* movups */
					}
					0x1f => {
						let opcode = if let Some(modrm) = bytes.first() {
							(modrm >> 3) & 7
						} else {
							return None;
						};
						match opcode {
							0x0 => {
								0x3a /* nop */
							}
							_ => return None,
						}
					}
					0x28 => {
						/* movaps */
  /* movapd */						0x53
					}
					0x29 => {
						/* movaps */
  /* movapd */						0x44
					}
					0x40 => {
						0x4b /* cmovo */
					}
					0x41 => {
						0x4b /* cmovno */
					}
					0x42 => {
						0x4b /* cmovb */
					}
					0x43 => {
						0x4b /* cmovae */
					}
					0x44 => {
						0x4b /* cmove */
					}
					0x45 => {
						0x4b /* cmovne */
					}
					0x46 => {
						0x4b /* cmovbe */
					}
					0x47 => {
						0x4b /* cmova */
					}
					0x48 => {
						0x4b /* cmovs */
					}
					0x49 => {
						0x4b /* cmovns */
					}
					0x4a => {
						0x4b /* cmovp */
					}
					0x4b => {
						0x4b /* cmovnp */
					}
					0x4c => {
						0x4b /* cmovl */
					}
					0x4d => {
						0x4b /* cmovge */
					}
					0x4e => {
						0x4b /* cmovle */
					}
					0x4f => {
						0x4b /* cmovg */
					}
					0x57 => {
						/* xorps */
  /* xorpd */						0x53
					}
					0x6c => {
						/* punpcklqdq */
						0x53
					}
					0x6d => {
						/* punpckhqdq */
						0x53
					}
					0x6e => {
						/* mov */
						0x51
					}
					0x6f => {
						/* movdqa */
  /* movdqu */						0x53
					}
					0x7e => {
						if prefix(0x66) {
							return 0x36; /* mov */
						}
						if prefix(0xf3) {
							return 0x50; /* movq */
						}
						return None;
					}
					0x7f => {
						/* movdqa */
  /* movdqu */						0x44
					}
					0x80 => {
						0x5 /* jo */
					}
					0x81 => {
						0x5 /* jno */
					}
					0x82 => {
						0x5 /* jb */
					}
					0x83 => {
						0x5 /* jae */
					}
					0x84 => {
						0x5 /* je */
					}
					0x85 => {
						0x5 /* jne */
					}
					0x86 => {
						0x5 /* jbe */
					}
					0x87 => {
						0x5 /* ja */
					}
					0x88 => {
						0x5 /* js */
					}
					0x89 => {
						0x5 /* jns */
					}
					0x8a => {
						0x5 /* jp */
					}
					0x8b => {
						0x5 /* jnp */
					}
					0x8c => {
						0x5 /* jl */
					}
					0x8d => {
						0x5 /* jge */
					}
					0x8e => {
						0x5 /* jle */
					}
					0x8f => {
						0x5 /* jg */
					}
					0x90 => {
						let opcode = if let Some(modrm) = bytes.first() {
							(modrm >> 3) & 7
						} else {
							return None;
						};
						match opcode {
							0x0 => {
								0x30 /* seto */
							}
							_ => return None,
						}
					}
					0x91 => {
						let opcode = if let Some(modrm) = bytes.first() {
							(modrm >> 3) & 7
						} else {
							return None;
						};
						match opcode {
							0x0 => {
								0x30 /* setno */
							}
							_ => return None,
						}
					}
					0x92 => {
						let opcode = if let Some(modrm) = bytes.first() {
							(modrm >> 3) & 7
						} else {
							return None;
						};
						match opcode {
							0x0 => {
								0x30 /* setb */
							}
							_ => return None,
						}
					}
					0x93 => {
						let opcode = if let Some(modrm) = bytes.first() {
							(modrm >> 3) & 7
						} else {
							return None;
						};
						match opcode {
							0x0 => {
								0x30 /* setae */
							}
							_ => return None,
						}
					}
					0x94 => {
						let opcode = if let Some(modrm) = bytes.first() {
							(modrm >> 3) & 7
						} else {
							return None;
						};
						match opcode {
							0x0 => {
								0x30 /* sete */
							}
							_ => return None,
						}
					}
					0x95 => {
						let opcode = if let Some(modrm) = bytes.first() {
							(modrm >> 3) & 7
						} else {
							return None;
						};
						match opcode {
							0x0 => {
								0x30 /* setne */
							}
							_ => return None,
						}
					}
					0x96 => {
						let opcode = if let Some(modrm) = bytes.first() {
							(modrm >> 3) & 7
						} else {
							return None;
						};
						match opcode {
							0x0 => {
								0x30 /* setbe */
							}
							_ => return None,
						}
					}
					0x97 => {
						let opcode = if let Some(modrm) = bytes.first() {
							(modrm >> 3) & 7
						} else {
							return None;
						};
						match opcode {
							0x0 => {
								0x30 /* seta */
							}
							_ => return None,
						}
					}
					0x98 => {
						let opcode = if let Some(modrm) = bytes.first() {
							(modrm >> 3) & 7
						} else {
							return None;
						};
						match opcode {
							0x0 => {
								0x30 /* sets */
							}
							_ => return None,
						}
					}
					0x99 => {
						let opcode = if let Some(modrm) = bytes.first() {
							(modrm >> 3) & 7
						} else {
							return None;
						};
						match opcode {
							0x0 => {
								0x30 /* setns */
							}
							_ => return None,
						}
					}
					0x9a => {
						let opcode = if let Some(modrm) = bytes.first() {
							(modrm >> 3) & 7
						} else {
							return None;
						};
						match opcode {
							0x0 => {
								0x30 /* setp */
							}
							_ => return None,
						}
					}
					0x9b => {
						let opcode = if let Some(modrm) = bytes.first() {
							(modrm >> 3) & 7
						} else {
							return None;
						};
						match opcode {
							0x0 => {
								0x30 /* setnp */
							}
							_ => return None,
						}
					}
					0x9c => {
						let opcode = if let Some(modrm) = bytes.first() {
							(modrm >> 3) & 7
						} else {
							return None;
						};
						match opcode {
							0x0 => {
								0x30 /* setl */
							}
							_ => return None,
						}
					}
					0x9d => {
						let opcode = if let Some(modrm) = bytes.first() {
							(modrm >> 3) & 7
						} else {
							return None;
						};
						match opcode {
							0x0 => {
								0x30 /* setge */
							}
							_ => return None,
						}
					}
					0x9e => {
						let opcode = if let Some(modrm) = bytes.first() {
							(modrm >> 3) & 7
						} else {
							return None;
						};
						match opcode {
							0x0 => {
								0x30 /* setle */
							}
							_ => return None,
						}
					}
					0x9f => {
						let opcode = if let Some(modrm) = bytes.first() {
							(modrm >> 3) & 7
						} else {
							return None;
						};
						match opcode {
							0x0 => {
								0x30 /* setg */
							}
							_ => return None,
						}
					}
					0xa3 => {
						0x40 /* bt */
					}
					0xab => {
						0x40 /* bts */
					}
					0xae => {
						let byte = if let Some((&byte, rest)) = bytes.split_first() {
							bytes = rest;
							b
						} else {
							return None;
						};
						match byte {
							0xf0 => {
								0x0 /* mfence */
							}
							_ => return None,
						}
					}
					0xaf => {
						0x4b /* imul */
					}
					0xb0 => {
						0x34 /* cmpxchg */
					}
					0xb1 => {
						0x40 /* cmpxchg */
					}
					0xb3 => {
						0x40 /* btr */
					}
					0xb6 => {
						0x48 /* movzx */
					}
					0xb7 => {
						0x49 /* movzx */
					}
					0xba => {
						let opcode = if let Some(modrm) = bytes.first() {
							(modrm >> 3) & 7
						} else {
							return None;
						};
						match opcode {
							0x4 => {
								0x3c /* bt */
							}
							0x5 => {
								0x3c /* bts */
							}
							0x6 => {
								0x3c /* btr */
							}
							0x7 => {
								0x3c /* btc */
							}
							_ => return None,
						}
					}
					0xbb => {
						0x40 /* btc */
					}
					0xbe => {
						0x48 /* movsx */
					}
					0xbf => {
						0x49 /* movsx */
					}
					0xc0 => {
						0x34 /* xadd */
					}
					0xc1 => {
						0x40 /* xadd */
					}
					0xd6 => {
						/* movq */
						0x42
					}
					_ => return None,
				}
			}
			0x10 => {
				0x34 /* adc */
			}
			0x11 => {
				0x40 /* adc */
			}
			0x12 => {
				0x46 /* adc */
			}
			0x13 => {
				0x4b /* adc */
			}
			0x14 => {
				0x7 /* adc */
			}
			0x15 => {
				0xa /* adc */
			}
			0x18 => {
				0x34 /* sbb */
			}
			0x19 => {
				0x40 /* sbb */
			}
			0x1a => {
				0x46 /* sbb */
			}
			0x1b => {
				0x4b /* sbb */
			}
			0x1c => {
				0x7 /* sbb */
			}
			0x1d => {
				0xa /* sbb */
			}
			0x20 => {
				0x34 /* and */
			}
			0x21 => {
				0x40 /* and */
			}
			0x22 => {
				0x46 /* and */
			}
			0x23 => {
				0x4b /* and */
			}
			0x24 => {
				0x7 /* and */
			}
			0x25 => {
				0xa /* and */
			}
			0x28 => {
				0x34 /* sub */
			}
			0x29 => {
				0x40 /* sub */
			}
			0x2a => {
				0x46 /* sub */
			}
			0x2b => {
				0x4b /* sub */
			}
			0x2c => {
				0x7 /* sub */
			}
			0x2d => {
				0xa /* sub */
			}
			0x30 => {
				0x34 /* xor */
			}
			0x31 => {
				0x40 /* xor */
			}
			0x32 => {
				0x46 /* xor */
			}
			0x33 => {
				0x4b /* xor */
			}
			0x34 => {
				0x7 /* xor */
			}
			0x35 => {
				0xa /* xor */
			}
			0x38 => {
				0x2f /* cmp */
			}
			0x39 => {
				0x39 /* cmp */
			}
			0x3a => {
				0x45 /* cmp */
			}
			0x3b => {
				0x47 /* cmp */
			}
			0x3c => {
				0x6 /* cmp */
			}
			0x3d => {
				0x9 /* cmp */
			}
			0x50 => {
				0x14 /* push */
			}
			0x51 => {
				0x17 /* push */
			}
			0x52 => {
				0x1a /* push */
			}
			0x53 => {
				0x1d /* push */
			}
			0x54 => {
				0x20 /* push */
			}
			0x55 => {
				0x23 /* push */
			}
			0x56 => {
				0x26 /* push */
			}
			0x57 => {
				0x29 /* push */
			}
			0x58 => {
				0x14 /* pop */
			}
			0x59 => {
				0x17 /* pop */
			}
			0x5a => {
				0x1a /* pop */
			}
			0x5b => {
				0x1d /* pop */
			}
			0x5c => {
				0x20 /* pop */
			}
			0x5d => {
				0x23 /* pop */
			}
			0x5e => {
				0x26 /* pop */
			}
			0x5f => {
				0x29 /* pop */
			}
			0x63 => {
				0x4a /* movsxd */
			}
			0x69 => {
				0x4e /* imul */
			}
			0x6b => {
				0x4d /* imul */
			}
			0x70 => {
				0x4 /* jo */
			}
			0x71 => {
				0x4 /* jno */
			}
			0x72 => {
				0x4 /* jb */
			}
			0x73 => {
				0x4 /* jae */
			}
			0x74 => {
				0x4 /* je */
			}
			0x75 => {
				0x4 /* jne */
			}
			0x76 => {
				0x4 /* jbe */
			}
			0x77 => {
				0x4 /* ja */
			}
			0x78 => {
				0x4 /* js */
			}
			0x79 => {
				0x4 /* jns */
			}
			0x7a => {
				0x4 /* jp */
			}
			0x7b => {
				0x4 /* jnp */
			}
			0x7c => {
				0x4 /* jl */
			}
			0x7d => {
				0x4 /* jge */
			}
			0x7e => {
				0x4 /* jle */
			}
			0x7f => {
				0x4 /* jg */
			}
			0x80 => {
				let opcode = if let Some(modrm) = bytes.first() {
					(modrm >> 3) & 7
				} else {
					return None;
				};
				match opcode {
					0x0 => {
						0x31 /* add */
					}
					0x1 => {
						0x31 /* or */
					}
					0x2 => {
						0x31 /* adc */
					}
					0x3 => {
						0x31 /* sbb */
					}
					0x4 => {
						0x31 /* and */
					}
					0x5 => {
						0x31 /* sub */
					}
					0x6 => {
						0x31 /* xor */
					}
					0x7 => {
						0x31 /* cmp */
					}
					_ => return None,
				}
			}
			0x81 => {
				let opcode = if let Some(modrm) = bytes.first() {
					(modrm >> 3) & 7
				} else {
					return None;
				};
				match opcode {
					0x0 => {
						0x3d /* add */
					}
					0x1 => {
						0x3d /* or */
					}
					0x2 => {
						0x3d /* adc */
					}
					0x3 => {
						0x3d /* sbb */
					}
					0x4 => {
						0x3d /* and */
					}
					0x5 => {
						0x3d /* sub */
					}
					0x6 => {
						0x3d /* xor */
					}
					0x7 => {
						0x3d /* cmp */
					}
					_ => return None,
				}
			}
			0x83 => {
				let opcode = if let Some(modrm) = bytes.first() {
					(modrm >> 3) & 7
				} else {
					return None;
				};
				match opcode {
					0x0 => {
						0x3c /* add */
					}
					0x1 => {
						0x3c /* or */
					}
					0x2 => {
						0x3c /* adc */
					}
					0x3 => {
						0x3c /* sbb */
					}
					0x4 => {
						0x3c /* and */
					}
					0x5 => {
						0x3c /* sub */
					}
					0x6 => {
						0x3c /* xor */
					}
					0x7 => {
						0x3c /* cmp */
					}
					_ => return None,
				}
			}
			0x84 => {
				0x2f /* test */
			}
			0x85 => {
				0x39 /* test */
			}
			0x86 => {
				0x34 /* xchg */
			}
			0x87 => {
				0x40 /* xchg */
			}
			0x88 => {
				0x34 /* mov */
			}
			0x89 => {
				0x41 /* mov */
			}
			0x8a => {
				0x46 /* mov */
			}
			0x8b => {
				0x4c /* mov */
			}
			0x8d => {
				0x4f /* lea */
			}
			0x90 => {
				/* nop */
  /* pause */				0x0
			}
			0x91 => {
				0xb /* xchg */
			}
			0x92 => {
				0xc /* xchg */
			}
			0x93 => {
				0xd /* xchg */
			}
			0x94 => {
				0xe /* xchg */
			}
			0x95 => {
				0xf /* xchg */
			}
			0x96 => {
				0x10 /* xchg */
			}
			0x97 => {
				0x11 /* xchg */
			}
			0x98 => {
				/* cwde */
  /* cbw */				0x1
			}
			0x99 => {
				/* cdq */
  /* cwd */				0x2
			}
			0xa0 => {
				0x8 /* mov */
			}
			0xa1 => {
				0x12 /* mov */
			}
			0xa2 => {
				0x2b /* mov */
			}
			0xa3 => {
				0x2c /* mov */
			}
			0xa4 => {
				0x0 /* movs */
			}
			0xa5 => {
				0x0 /* movs */
			}
			0xa8 => {
				0x6 /* test */
			}
			0xa9 => {
				0x9 /* test */
			}
			0xb0 => {
				0x13 /* mov */
			}
			0xb1 => {
				0x16 /* mov */
			}
			0xb2 => {
				0x19 /* mov */
			}
			0xb3 => {
				0x1c /* mov */
			}
			0xb4 => {
				0x1f /* mov */
			}
			0xb5 => {
				0x22 /* mov */
			}
			0xb6 => {
				0x25 /* mov */
			}
			0xb7 => {
				0x28 /* mov */
			}
			0xb8 => {
				0x15 /* mov */
			}
			0xb9 => {
				0x18 /* mov */
			}
			0xba => {
				0x1b /* mov */
			}
			0xbb => {
				0x1e /* mov */
			}
			0xbc => {
				0x21 /* mov */
			}
			0xbd => {
				0x24 /* mov */
			}
			0xbe => {
				0x27 /* mov */
			}
			0xbf => {
				0x2a /* mov */
			}
			0xc0 => {
				let opcode = if let Some(modrm) = bytes.first() {
					(modrm >> 3) & 7
				} else {
					return None;
				};
				match opcode {
					0x0 => {
						0x31 /* rol */
					}
					0x1 => {
						0x31 /* ror */
					}
					0x2 => {
						0x31 /* rcl */
					}
					0x3 => {
						0x31 /* rcr */
					}
					0x4 => {
						0x31 /* shl */
					}
					0x5 => {
						0x31 /* shr */
					}
					0x7 => {
						0x31 /* sar */
					}
					_ => return None,
				}
			}
			0xc1 => {
				let opcode = if let Some(modrm) = bytes.first() {
					(modrm >> 3) & 7
				} else {
					return None;
				};
				match opcode {
					0x0 => {
						0x3b /* rol */
					}
					0x1 => {
						0x3b /* ror */
					}
					0x2 => {
						0x3b /* rcl */
					}
					0x3 => {
						0x3b /* rcr */
					}
					0x4 => {
						0x3b /* shl */
					}
					0x5 => {
						0x3b /* shr */
					}
					0x7 => {
						0x3b /* sar */
					}
					_ => return None,
				}
			}
			0xc3 => {
				0x0 /* ret */
			}
			0xc6 => {
				let opcode = if let Some(modrm) = bytes.first() {
					(modrm >> 3) & 7
				} else {
					return None;
				};
				match opcode {
					0x0 => {
						0x31 /* mov */
					}
					_ => return None,
				}
			}
			0xc7 => {
				let opcode = if let Some(modrm) = bytes.first() {
					(modrm >> 3) & 7
				} else {
					return None;
				};
				match opcode {
					0x0 => {
						0x3d /* mov */
					}
					_ => return None,
				}
			}
			0xcc => {
				0x0 /* int3 */
			}
			0xcd => {
				0x3 /* int */
			}
			0xd0 => {
				let opcode = if let Some(modrm) = bytes.first() {
					(modrm >> 3) & 7
				} else {
					return None;
				};
				match opcode {
					0x0 => {
						0x32 /* rol */
					}
					0x1 => {
						0x32 /* ror */
					}
					0x2 => {
						0x32 /* rcl */
					}
					0x3 => {
						0x32 /* rcr */
					}
					0x4 => {
						0x32 /* shl */
					}
					0x5 => {
						0x32 /* shr */
					}
					0x7 => {
						0x32 /* sar */
					}
					_ => return None,
				}
			}
			0xd1 => {
				let opcode = if let Some(modrm) = bytes.first() {
					(modrm >> 3) & 7
				} else {
					return None;
				};
				match opcode {
					0x0 => {
						0x3e /* rol */
					}
					0x1 => {
						0x3e /* ror */
					}
					0x2 => {
						0x3e /* rcl */
					}
					0x3 => {
						0x3e /* rcr */
					}
					0x4 => {
						0x3e /* shl */
					}
					0x5 => {
						0x3e /* shr */
					}
					0x7 => {
						0x3e /* sar */
					}
					_ => return None,
				}
			}
			0xd2 => {
				let opcode = if let Some(modrm) = bytes.first() {
					(modrm >> 3) & 7
				} else {
					return None;
				};
				match opcode {
					0x0 => {
						0x33 /* rol */
					}
					0x1 => {
						0x33 /* ror */
					}
					0x2 => {
						0x33 /* rcl */
					}
					0x3 => {
						0x33 /* rcr */
					}
					0x4 => {
						0x33 /* shl */
					}
					0x5 => {
						0x33 /* shr */
					}
					0x7 => {
						0x33 /* sar */
					}
					_ => return None,
				}
			}
			0xd3 => {
				let opcode = if let Some(modrm) = bytes.first() {
					(modrm >> 3) & 7
				} else {
					return None;
				};
				match opcode {
					0x0 => {
						0x3f /* rol */
					}
					0x1 => {
						0x3f /* ror */
					}
					0x2 => {
						0x3f /* rcl */
					}
					0x3 => {
						0x3f /* rcr */
					}
					0x4 => {
						0x3f /* shl */
					}
					0x5 => {
						0x3f /* shr */
					}
					0x7 => {
						0x3f /* sar */
					}
					_ => return None,
				}
			}
			0xe8 => {
				0x5 /* call */
			}
			0xe9 => {
				0x5 /* jmp */
			}
			0xeb => {
				0x4 /* jmp */
			}
			0xf6 => {
				let opcode = if let Some(modrm) = bytes.first() {
					(modrm >> 3) & 7
				} else {
					return None;
				};
				match opcode {
					0x0 => {
						0x2e /* test */
					}
					0x2 => {
						0x30 /* not */
					}
					0x3 => {
						0x30 /* neg */
					}
					0x4 => {
						0x2d /* mul */
					}
					0x5 => {
						0x2d /* imul */
					}
					0x6 => {
						0x2d /* div */
					}
					0x7 => {
						0x2d /* idiv */
					}
					_ => return None,
				}
			}
			0xf7 => {
				let opcode = if let Some(modrm) = bytes.first() {
					(modrm >> 3) & 7
				} else {
					return None;
				};
				match opcode {
					0x0 => {
						0x38 /* test */
					}
					0x2 => {
						0x3a /* not */
					}
					0x3 => {
						0x3a /* neg */
					}
					0x4 => {
						0x37 /* mul */
					}
					0x5 => {
						0x37 /* imul */
					}
					0x6 => {
						0x37 /* div */
					}
					0x7 => {
						0x37 /* idiv */
					}
					_ => return None,
				}
			}
			0xfe => {
				let opcode = if let Some(modrm) = bytes.first() {
					(modrm >> 3) & 7
				} else {
					return None;
				};
				match opcode {
					0x0 => {
						0x30 /* inc */
					}
					0x1 => {
						0x30 /* dec */
					}
					_ => return None,
				}
			}
			0xff => {
				let opcode = if let Some(modrm) = bytes.first() {
					(modrm >> 3) & 7
				} else {
					return None;
				};
				match opcode {
					0x0 => {
						0x3a /* inc */
					}
					0x1 => {
						0x3a /* dec */
					}
					0x2 => {
						0x35 /* call */
					}
					0x4 => {
						0x35 /* jmp */
					}
					_ => return None,
				}
			}
			_ => return None,
		}
	})
}
