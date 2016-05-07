pub fn decode(mut bytes: &[u8], prefixes: u32) -> u32 {
	let byte = if let Some((&byte, rest)) = bytes.split_first() {
		bytes = rest;
		byte
	} else {
		return 0;
	};
	match byte {
		0x0 => {
			/* add */

			0x10101
		}
		0x1 => {
			/* add */

			0x10111
		}
		0x2 => {
			/* add */

			0x10201
		}
		0x3 => {
			/* add */

			0x10211
		}
		0x4 => {
			/* add */

			0x21241
		}
		0x5 => {
			/* add */

			0x21291
		}
		0x8 => {
			/* or */

			0x10101
		}
		0x9 => {
			/* or */

			0x10111
		}
		0xa => {
			/* or */

			0x10201
		}
		0xb => {
			/* or */

			0x10211
		}
		0xc => {
			/* or */

			0x21241
		}
		0xd => {
			/* or */

			0x21291
		}
		0xf => {
			let byte = if let Some((&byte, rest)) = bytes.split_first() {
				bytes = rest;
				byte
			} else {
				return 0;
			};
			match byte {
				0xb => {
					/* ud2 */

					0x1110
				}
				0x10 => {
					if prefixes | 8 != 0 {
						return  /* movupd */ 
0x10218;
					}
					if prefixes | 4 != 0 {
						return  /* movsd */ 
0x10214;
					} /* movups */
					0x10210
				}
				0x11 => {
					if prefixes | 8 != 0 {
						return  /* movupd */ 
0x10118;
					}
					if prefixes | 4 != 0 {
						return  /* movsd */ 
0x10114;
					} /* movups */
					0x10110
				}
				0x1f => {
					let opcode = if let Some(modrm) = bytes.first() {
						(modrm >> 3u8) & 7
					} else {
						return 0;
					};
					match opcode { 
						0x0 => {
							/* nop */

							0x10118
						}
						// 1 => capstone: nop dword ptr [rdx] (length: 3)
						// 2 => capstone: nop dword ptr [rdx] (length: 3)
						// 3 => capstone: nop dword ptr [rdx] (length: 3)
						// 4 => capstone: nop dword ptr [rdx] (length: 3)
						// 5 => capstone: nop dword ptr [rdx] (length: 3)
						// 6 => capstone: nop dword ptr [rdx] (length: 3)
						// 7 => capstone: nop dword ptr [rdx] (length: 3)
						_ => 0,
					}
				}
				0x28 => {
					if prefixes | 8 != 0 {
						return  /* movapd */ 
0x10218;
					} /* movaps */
					0x10210
				}
				0x29 => {
					if prefixes | 8 != 0 {
						return  /* movapd */ 
0x10118;
					} /* movaps */
					0x10110
				}
				0x40 => {
					/* cmovo */

					0x10210
				}
				0x41 => {
					/* cmovno */

					0x10210
				}
				0x42 => {
					/* cmovb */

					0x10210
				}
				0x43 => {
					/* cmovae */

					0x10210
				}
				0x44 => {
					/* cmove */

					0x10210
				}
				0x45 => {
					/* cmovne */

					0x10210
				}
				0x46 => {
					/* cmovbe */

					0x10210
				}
				0x47 => {
					/* cmova */

					0x10210
				}
				0x48 => {
					/* cmovs */

					0x10210
				}
				0x49 => {
					/* cmovns */

					0x10210
				}
				0x4a => {
					/* cmovp */

					0x10210
				}
				0x4b => {
					/* cmovnp */

					0x10210
				}
				0x4c => {
					/* cmovl */

					0x10210
				}
				0x4d => {
					/* cmovge */

					0x10210
				}
				0x4e => {
					/* cmovle */

					0x10210
				}
				0x4f => {
					/* cmovg */

					0x10210
				}
				0x57 => {
					if prefixes | 8 != 0 {
						return  /* xorpd */ 
0x10218;
					} /* xorps */
					0x10210
				}
				0x6c => {
					// Multiple prefixes
					// punpcklqdq

					0x10218
				}
				0x6d => {
					// Multiple prefixes
					// punpckhqdq

					0x10218
				}
				0x6e => {
					// Multiple prefixes
					// mov

					0x10218
				}
				0x6f => {
					if prefixes | 8 != 0 {
						return  /* movdqa */ 
0x10218;
					}
					if prefixes | 2 != 0 {
						return  /* movdqu */ 
0x10212;
					}
					0
				}
				0x7e => {
					if prefixes | 8 != 0 {
						return  /* mov */ 
0x10118;
					}
					if prefixes | 2 != 0 {
						return  /* movq */ 
0x10212;
					}
					0
				}
				0x7f => {
					if prefixes | 8 != 0 {
						return  /* movdqa */ 
0x10118;
					}
					if prefixes | 2 != 0 {
						return  /* movdqu */ 
0x10112;
					}
					0
				}
				0x80 => {
					/* jo */

					0xf10
				}
				0x81 => {
					/* jno */

					0xf10
				}
				0x82 => {
					/* jb */

					0xf10
				}
				0x83 => {
					/* jae */

					0xf10
				}
				0x84 => {
					/* je */

					0xf10
				}
				0x85 => {
					/* jne */

					0xf10
				}
				0x86 => {
					/* jbe */

					0xf10
				}
				0x87 => {
					/* ja */

					0xf10
				}
				0x88 => {
					/* js */

					0xf10
				}
				0x89 => {
					/* jns */

					0xf10
				}
				0x8a => {
					/* jp */

					0xf10
				}
				0x8b => {
					/* jnp */

					0xf10
				}
				0x8c => {
					/* jl */

					0xf10
				}
				0x8d => {
					/* jge */

					0xf10
				}
				0x8e => {
					/* jle */

					0xf10
				}
				0x8f => {
					/* jg */

					0xf10
				}
				0x90 => {
					/* seto */

					0x10100
				}
				0x91 => {
					/* setno */

					0x10100
				}
				0x92 => {
					/* setb */

					0x10100
				}
				0x93 => {
					/* setae */

					0x10100
				}
				0x94 => {
					/* sete */

					0x10100
				}
				0x95 => {
					/* setne */

					0x10100
				}
				0x96 => {
					/* setbe */

					0x10100
				}
				0x97 => {
					/* seta */

					0x10100
				}
				0x98 => {
					/* sets */

					0x10100
				}
				0x99 => {
					/* setns */

					0x10100
				}
				0x9a => {
					/* setp */

					0x10100
				}
				0x9b => {
					/* setnp */

					0x10100
				}
				0x9c => {
					/* setl */

					0x10100
				}
				0x9d => {
					/* setge */

					0x10100
				}
				0x9e => {
					/* setle */

					0x10100
				}
				0x9f => {
					/* setg */

					0x10100
				}
				0xa3 => {
					/* bt */

					0x10110
				}
				0xab => {
					/* bts */

					0x10111
				}
				0xae => {
					let byte = if let Some((&byte, rest)) = bytes.split_first() {
						bytes = rest;
						byte
					} else {
						return 0;
					};
					match byte {
						0xf0 => {
							/* mfence */

							0x1210
						}
						_ => 0,
					}
				}
				0xaf => {
					/* imul */

					0x10210
				}
				0xb0 => {
					/* cmpxchg */

					0x10101
				}
				0xb1 => {
					/* cmpxchg */

					0x10111
				}
				0xb3 => {
					/* btr */

					0x10111
				}
				0xb6 => {
					/* movzx */

					0x10200
				}
				0xb7 => {
					/* movzx */

					0x10210
				}
				0xba => {
					let opcode = if let Some(modrm) = bytes.first() {
						(modrm >> 3u8) & 7
					} else {
						return 0;
					};
					match opcode { 
						// 0 => capstone: unknown
						// 1 => capstone: unknown
						// 2 => capstone: unknown
						// 3 => capstone: unknown
						0x4 => {
							/* bt */

							0x10150
						}
						0x5 => {
							/* bts */

							0x10151
						}
						0x6 => {
							/* btr */

							0x10151
						}
						0x7 => {
							/* btc */

							0x10151
						}
						_ => 0,
					}
				}
				0xbb => {
					/* btc */

					0x10111
				}
				0xbe => {
					/* movsx */

					0x10200
				}
				0xbf => {
					/* movsx */

					0x10210
				}
				0xc0 => {
					/* xadd */

					0x10101
				}
				0xc1 => {
					/* xadd */

					0x10111
				}
				0xd6 => {
					// Multiple prefixes
					// movq

					0x10118
				}
				_ => 0,
			}
		}
		0x10 => {
			/* adc */

			0x10101
		}
		0x11 => {
			/* adc */

			0x10111
		}
		0x12 => {
			/* adc */

			0x10201
		}
		0x13 => {
			/* adc */

			0x10211
		}
		0x14 => {
			/* adc */

			0x21241
		}
		0x15 => {
			/* adc */

			0x21291
		}
		0x18 => {
			/* sbb */

			0x10101
		}
		0x19 => {
			/* sbb */

			0x10111
		}
		0x1a => {
			/* sbb */

			0x10201
		}
		0x1b => {
			/* sbb */

			0x10211
		}
		0x1c => {
			/* sbb */

			0x21241
		}
		0x1d => {
			/* sbb */

			0x21291
		}
		0x20 => {
			/* and */

			0x10101
		}
		0x21 => {
			/* and */

			0x10611
		}
		0x22 => {
			/* and */

			0x10201
		}
		0x23 => {
			/* and */

			0x10711
		}
		0x24 => {
			/* and */

			0x21241
		}
		0x25 => {
			/* and */

			0x21291
		}
		0x28 => {
			/* sub */

			0x10101
		}
		0x29 => {
			/* sub */

			0x10111
		}
		0x2a => {
			/* sub */

			0x10201
		}
		0x2b => {
			/* sub */

			0x10211
		}
		0x2c => {
			/* sub */

			0x21241
		}
		0x2d => {
			/* sub */

			0x21291
		}
		0x30 => {
			/* xor */

			0x10101
		}
		0x31 => {
			/* xor */

			0x10111
		}
		0x32 => {
			/* xor */

			0x10201
		}
		0x33 => {
			/* xor */

			0x10211
		}
		0x34 => {
			/* xor */

			0x21241
		}
		0x35 => {
			/* xor */

			0x21291
		}
		0x38 => {
			/* cmp */

			0x10300
		}
		0x39 => {
			/* cmp */

			0x10310
		}
		0x3a => {
			/* cmp */

			0x10300
		}
		0x3b => {
			/* cmp */

			0x10310
		}
		0x3c => {
			/* cmp */

			0x1240
		}
		0x3d => {
			/* cmp */

			0x1290
		}
		0x50 => {
			/* push */

			0x910
		}
		0x51 => {
			/* push */

			0x910
		}
		0x52 => {
			/* push */

			0x910
		}
		0x53 => {
			/* push */

			0x910
		}
		0x54 => {
			/* push */

			0x910
		}
		0x55 => {
			/* push */

			0x910
		}
		0x56 => {
			/* push */

			0x910
		}
		0x57 => {
			/* push */

			0x910
		}
		0x58 => {
			/* pop */

			0xa10
		}
		0x59 => {
			/* pop */

			0xa10
		}
		0x5a => {
			/* pop */

			0xa10
		}
		0x5b => {
			/* pop */

			0xa10
		}
		0x5c => {
			/* pop */

			0xa10
		}
		0x5d => {
			/* pop */

			0xa10
		}
		0x5e => {
			/* pop */

			0xa10
		}
		0x5f => {
			/* pop */

			0xa10
		}
		0x63 => {
			/* movsxd */

			0x10210
		}
		0x69 => {
			/* imul */

			0x10290
		}
		0x6b => {
			/* imul */

			0x10250
		}
		0x70 => {
			/* jo */

			0x1010
		}
		0x71 => {
			/* jno */

			0x1010
		}
		0x72 => {
			/* jb */

			0x1010
		}
		0x73 => {
			/* jae */

			0x1010
		}
		0x74 => {
			/* je */

			0x1010
		}
		0x75 => {
			/* jne */

			0x1010
		}
		0x76 => {
			/* jbe */

			0x1010
		}
		0x77 => {
			/* ja */

			0x1010
		}
		0x78 => {
			/* js */

			0x1010
		}
		0x79 => {
			/* jns */

			0x1010
		}
		0x7a => {
			/* jp */

			0x1010
		}
		0x7b => {
			/* jnp */

			0x1010
		}
		0x7c => {
			/* jl */

			0x1010
		}
		0x7d => {
			/* jge */

			0x1010
		}
		0x7e => {
			/* jle */

			0x1010
		}
		0x7f => {
			/* jg */

			0x1010
		}
		0x80 => {
			// Opcode table

			// 0 => add

			// 1 => or

			// 2 => adc

			// 3 => sbb

			// 4 => and

			// 5 => sub

			// 6 => xor

			// 7 => cmp
			0x10140
		}
		0x81 => {
			// Opcode table

			// 0 => add

			// 1 => or

			// 2 => adc

			// 3 => sbb

			// 4 => and

			// 5 => sub

			// 6 => xor

			// 7 => cmp
			0x10190
		}
		0x83 => {
			// Opcode table

			// 0 => add

			// 1 => or

			// 2 => adc

			// 3 => sbb

			// 4 => and

			// 5 => sub

			// 6 => xor

			// 7 => cmp
			0x10150
		}
		0x84 => {
			/* test */

			0x10300
		}
		0x85 => {
			/* test */

			0x10310
		}
		0x86 => {
			/* xchg */

			0x10100
		}
		0x87 => {
			/* xchg */

			0x10110
		}
		0x88 => {
			/* mov */

			0x10100
		}
		0x89 => {
			/* mov */

			0x10410
		}
		0x8a => {
			/* mov */

			0x10200
		}
		0x8b => {
			/* mov */

			0x10510
		}
		0x8d => {
			/* lea */

			0x10810
		}
		0x90 => {
			if prefixes | 2 != 0 {
				return  /* pause */ 
0x1212;
			} /* nop */
			0x1218
		}
		0x91 => {
			/* xchg */

			0x21210
		}
		0x92 => {
			/* xchg */

			0x21210
		}
		0x93 => {
			/* xchg */

			0x21210
		}
		0x94 => {
			/* xchg */

			0x21210
		}
		0x95 => {
			/* xchg */

			0x21210
		}
		0x96 => {
			/* xchg */

			0x21210
		}
		0x97 => {
			/* xchg */

			0x21210
		}
		0x98 => {
			if prefixes | 8 != 0 {
				return  /* cbw */ 
0x21218;
			} /* cwde */
			0x21210
		}
		0x99 => {
			if prefixes | 8 != 0 {
				return  /* cwd */ 
0x41218;
			} /* cdq */
			0x41210
		}
		0xa0 => {
			/* mov */

			0x20c00
		}
		0xa1 => {
			/* mov */

			0x20c10
		}
		0xa2 => {
			/* mov */

			0xc00
		}
		0xa3 => {
			/* mov */

			0xc10
		}
		0xa4 => {
			/* movs */

			0x1200
		}
		0xa5 => {
			/* movs */

			0x1210
		}
		0xa8 => {
			/* test */

			0x1240
		}
		0xa9 => {
			/* test */

			0x1290
		}
		0xb0 => {
			/* mov */

			0xb40
		}
		0xb1 => {
			/* mov */

			0xb40
		}
		0xb2 => {
			/* mov */

			0xb40
		}
		0xb3 => {
			/* mov */

			0xb40
		}
		0xb4 => {
			/* mov */

			0xb40
		}
		0xb5 => {
			/* mov */

			0xb40
		}
		0xb6 => {
			/* mov */

			0xb40
		}
		0xb7 => {
			/* mov */

			0xb40
		}
		0xb8 => {
			/* mov */

			0xbd0
		}
		0xb9 => {
			/* mov */

			0xbd0
		}
		0xba => {
			/* mov */

			0xbd0
		}
		0xbb => {
			/* mov */

			0xbd0
		}
		0xbc => {
			/* mov */

			0xbd0
		}
		0xbd => {
			/* mov */

			0xbd0
		}
		0xbe => {
			/* mov */

			0xbd0
		}
		0xbf => {
			/* mov */

			0xbd0
		}
		0xc0 => {
			let opcode = if let Some(modrm) = bytes.first() {
				(modrm >> 3u8) & 7
			} else {
				return 0;
			};
			match opcode { 
				0x0 => {
					/* rol */

					0x10140
				}
				0x1 => {
					/* ror */

					0x10140
				}
				0x2 => {
					/* rcl */

					0x10140
				}
				0x3 => {
					/* rcr */

					0x10140
				}
				0x4 => {
					/* shl */

					0x10140
				}
				0x5 => {
					/* shr */

					0x10140
				}
				// 6 => capstone: rcr byte ptr [rdx], 0x1a (length: 3)
				0x7 => {
					/* sar */

					0x10140
				}
				_ => 0,
			}
		}
		0xc1 => {
			let opcode = if let Some(modrm) = bytes.first() {
				(modrm >> 3u8) & 7
			} else {
				return 0;
			};
			match opcode { 
				0x0 => {
					/* rol */

					0x10140
				}
				0x1 => {
					/* ror */

					0x10140
				}
				0x2 => {
					/* rcl */

					0x10140
				}
				0x3 => {
					/* rcr */

					0x10140
				}
				0x4 => {
					/* shl */

					0x10140
				}
				0x5 => {
					/* shr */

					0x10140
				}
				// 6 => capstone: rcr dword ptr [rdx], 0x1a (length: 3)
				0x7 => {
					/* sar */

					0x10140
				}
				_ => 0,
			}
		}
		0xc3 => {
			/* ret */

			0x1210
		}
		0xc6 => {
			// Opcode table

			// 0 => mov

			// 1 => capstone: unknown

			// 2 => capstone: unknown

			// 3 => capstone: unknown

			// 4 => capstone: unknown

			// 5 => capstone: unknown

			// 6 => capstone: unknown

			// 7 => capstone: unknown
			0x10140
		}
		0xc7 => {
			// Opcode table

			// 0 => mov

			// 1 => capstone: unknown

			// 2 => capstone: unknown

			// 3 => capstone: unknown

			// 4 => capstone: unknown

			// 5 => capstone: unknown

			// 6 => capstone: unknown

			// 7 => capstone: unknown
			0x10190
		}
		0xcc => {
			/* int3 */

			0x1210
		}
		0xcd => {
			/* int */

			0x1240
		}
		0xd0 => {
			let opcode = if let Some(modrm) = bytes.first() {
				(modrm >> 3u8) & 7
			} else {
				return 0;
			};
			match opcode { 
				0x0 => {
					/* rol */

					0x10100
				}
				0x1 => {
					/* ror */

					0x10100
				}
				0x2 => {
					/* rcl */

					0x10100
				}
				0x3 => {
					/* rcr */

					0x10100
				}
				0x4 => {
					/* shl */

					0x10100
				}
				0x5 => {
					/* shr */

					0x10100
				}
				// 6 => capstone: rcr byte ptr [rdx], 1 (length: 2)
				0x7 => {
					/* sar */

					0x10100
				}
				_ => 0,
			}
		}
		0xd1 => {
			let opcode = if let Some(modrm) = bytes.first() {
				(modrm >> 3u8) & 7
			} else {
				return 0;
			};
			match opcode { 
				0x0 => {
					/* rol */

					0x10110
				}
				0x1 => {
					/* ror */

					0x10110
				}
				0x2 => {
					/* rcl */

					0x10110
				}
				0x3 => {
					/* rcr */

					0x10110
				}
				0x4 => {
					/* shl */

					0x10110
				}
				0x5 => {
					/* shr */

					0x10110
				}
				// 6 => capstone: rcr dword ptr [rdx], 1 (length: 2)
				0x7 => {
					/* sar */

					0x10110
				}
				_ => 0,
			}
		}
		0xd2 => {
			let opcode = if let Some(modrm) = bytes.first() {
				(modrm >> 3u8) & 7
			} else {
				return 0;
			};
			match opcode { 
				0x0 => {
					/* rol */

					0x10100
				}
				0x1 => {
					/* ror */

					0x10100
				}
				0x2 => {
					/* rcl */

					0x10100
				}
				0x3 => {
					/* rcr */

					0x10100
				}
				0x4 => {
					/* shl */

					0x10100
				}
				0x5 => {
					/* shr */

					0x10100
				}
				// 6 => capstone: rcr byte ptr [rdx], cl (length: 2)
				0x7 => {
					/* sar */

					0x10100
				}
				_ => 0,
			}
		}
		0xd3 => {
			let opcode = if let Some(modrm) = bytes.first() {
				(modrm >> 3u8) & 7
			} else {
				return 0;
			};
			match opcode { 
				0x0 => {
					/* rol */

					0x10100
				}
				0x1 => {
					/* ror */

					0x10100
				}
				0x2 => {
					/* rcl */

					0x10100
				}
				0x3 => {
					/* rcr */

					0x10100
				}
				0x4 => {
					/* shl */

					0x10100
				}
				0x5 => {
					/* shr */

					0x10100
				}
				// 6 => capstone: rcr dword ptr [rdx], cl (length: 2)
				0x7 => {
					/* sar */

					0x10100
				}
				_ => 0,
			}
		}
		0xe8 => {
			/* call */

			0xe10
		}
		0xe9 => {
			/* jmp */

			0xf10
		}
		0xeb => {
			/* jmp */

			0x1010
		}
		0xf6 => {
			let opcode = if let Some(modrm) = bytes.first() {
				(modrm >> 3u8) & 7
			} else {
				return 0;
			};
			match opcode { 
				0x0 => {
					/* test */

					0x10340
				}
				// 1 => capstone: neg byte ptr [rdx] (length: 2)
				0x2 => {
					/* not */

					0x10101
				}
				0x3 => {
					/* neg */

					0x10101
				}
				0x4 => {
					/* mul */

					0x30300
				}
				0x5 => {
					/* imul */

					0x30300
				}
				0x6 => {
					/* div */

					0x30300
				}
				0x7 => {
					/* idiv */

					0x30300
				}
				_ => 0,
			}
		}
		0xf7 => {
			let opcode = if let Some(modrm) = bytes.first() {
				(modrm >> 3u8) & 7
			} else {
				return 0;
			};
			match opcode { 
				0x0 => {
					/* test */

					0x10390
				}
				// 1 => capstone: neg dword ptr [rdx] (length: 2)
				0x2 => {
					/* not */

					0x10111
				}
				0x3 => {
					/* neg */

					0x10111
				}
				0x4 => {
					/* mul */

					0x70310
				}
				0x5 => {
					/* imul */

					0x70310
				}
				0x6 => {
					/* div */

					0x70310
				}
				0x7 => {
					/* idiv */

					0x70310
				}
				_ => 0,
			}
		}
		0xfe => {
			// Opcode table

			// 0 => inc

			// 1 => dec

			// 2 => capstone: unknown

			// 3 => capstone: unknown

			// 4 => capstone: unknown

			// 5 => capstone: unknown

			// 6 => capstone: unknown

			// 7 => capstone: unknown
			0x10101
		}
		0xff => {
			let opcode = if let Some(modrm) = bytes.first() {
				(modrm >> 3u8) & 7
			} else {
				return 0;
			};
			match opcode { 
				0x0 => {
					/* inc */

					0x10111
				}
				0x1 => {
					/* dec */

					0x10111
				}
				0x2 => {
					/* call */

					0x10110
				}
				// 3 => capstone: lcall ptr [rdx] (length: 2)
				0x4 => {
					/* jmp */

					0x10110
				}
				// 5 => capstone: lcall ptr [rdx] (length: 2)
				// 6 => capstone: lcall ptr [rdx] (length: 2)
				// 7 => capstone: lcall ptr [rdx] (length: 2)
				_ => 0,
			}
		}
		_ => 0,
	}
}
