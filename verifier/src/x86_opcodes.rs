use x86_decoder::Cursor;
pub fn decode(c: &mut Cursor, prefixes: u32) -> u32 {
	match c.next() {
		0x0 => {
			/* add */

			0x20201
		}
		0x1 => {
			/* add */

			0x20219
		}
		0x2 => {
			/* add */

			0x20401
		}
		0x3 => {
			/* add */

			0x20419
		}
		0x4 => {
			/* add */

			0x42481
		}
		0x5 => {
			/* add */

			0x42519
		}
		0x8 => {
			/* or */

			0x20201
		}
		0x9 => {
			/* or */

			0x20219
		}
		0xa => {
			/* or */

			0x20401
		}
		0xb => {
			/* or */

			0x20419
		}
		0xc => {
			/* or */

			0x42481
		}
		0xd => {
			/* or */

			0x42519
		}
		0xf => {
			match c.next() {
				0xb => {
					/* ud2 */

					0x2210
				}
				0x10 => {
					if prefixes & 8 != 0 {
						return  /* movupd */ 
0x20448;
					}
					if prefixes & 4 != 0 {
						return  /* movsd */ 
0x20434;
					} /* movups */
					0x20440
				}
				0x11 => {
					if prefixes & 8 != 0 {
						return  /* movupd */ 
0x20248;
					}
					if prefixes & 4 != 0 {
						return  /* movsd */ 
0x20234;
					} /* movups */
					0x20240
				}
				0x1f => {
					match (c.peek() >> 3u8) & 7 { 
						0x0 => {
							/* nop */

							0x20218
						}
						// 1 => capstone: nop dword ptr [rdx]
						// 2 => capstone: nop dword ptr [rdx]
						// 3 => capstone: nop dword ptr [rdx]
						// 4 => capstone: nop dword ptr [rdx]
						// 5 => capstone: nop dword ptr [rdx]
						// 6 => capstone: nop dword ptr [rdx]
						// 7 => capstone: nop dword ptr [rdx]
						_ => 0,
					}
				}
				0x28 => {
					if prefixes & 8 != 0 {
						return  /* movapd */ 
0x20448;
					} /* movaps */
					0x20440
				}
				0x29 => {
					if prefixes & 8 != 0 {
						return  /* movapd */ 
0x20248;
					} /* movaps */
					0x20240
				}
				0x2a => {
					if prefixes & 4 != 0 {
						return  /* cvtsi2sd */ 
0x20414;
					}
					if prefixes & 2 != 0 {
						return  /* cvtsi2ss */ 
0x20412;
					}
					0
				}
				0x2c => {
					if prefixes & 4 != 0 {
						return  /* cvttsd2si */ 
0x20434;
					}
					if prefixes & 2 != 0 {
						return  /* cvttss2si */ 
0x20412;
					}
					0
				}
				0x2e => {
					if prefixes & 8 != 0 {
						return  /* ucomisd */ 
0x20638;
					} /* ucomiss */
					0x20610
				}
				0x38 => {
					match c.next() {
						0x0 => {
							// Multiple prefixes
							// pshufb

							0x20448
						}
						_ => 0,
					}
				}
				0x40 => {
					/* cmovo */

					0x20410
				}
				0x41 => {
					/* cmovno */

					0x20410
				}
				0x42 => {
					/* cmovb */

					0x20410
				}
				0x43 => {
					/* cmovae */

					0x20410
				}
				0x44 => {
					/* cmove */

					0x20410
				}
				0x45 => {
					/* cmovne */

					0x20410
				}
				0x46 => {
					/* cmovbe */

					0x20410
				}
				0x47 => {
					/* cmova */

					0x20410
				}
				0x48 => {
					/* cmovs */

					0x20410
				}
				0x49 => {
					/* cmovns */

					0x20410
				}
				0x4a => {
					/* cmovp */

					0x20410
				}
				0x4b => {
					/* cmovnp */

					0x20410
				}
				0x4c => {
					/* cmovl */

					0x20410
				}
				0x4d => {
					/* cmovge */

					0x20410
				}
				0x4e => {
					/* cmovle */

					0x20410
				}
				0x4f => {
					/* cmovg */

					0x20410
				}
				0x54 => {
					if prefixes & 8 != 0 {
						return  /* andpd */ 
0x20448;
					} /* andps */
					0x20440
				}
				0x57 => {
					if prefixes & 8 != 0 {
						return  /* xorpd */ 
0x20448;
					} /* xorps */
					0x20440
				}
				0x58 => {
					if prefixes & 8 != 0 {
						return  /* addpd */ 
0x20448;
					}
					if prefixes & 4 != 0 {
						return  /* addsd */ 
0x20434;
					}
					if prefixes & 2 != 0 {
						return  /* addss */ 
0x20412;
					} /* addps */
					0x20440
				}
				0x59 => {
					if prefixes & 8 != 0 {
						return  /* mulpd */ 
0x20448;
					}
					if prefixes & 4 != 0 {
						return  /* mulsd */ 
0x20434;
					}
					if prefixes & 2 != 0 {
						return  /* mulss */ 
0x20412;
					} /* mulps */
					0x20440
				}
				0x5c => {
					if prefixes & 8 != 0 {
						return  /* subpd */ 
0x20448;
					}
					if prefixes & 4 != 0 {
						return  /* subsd */ 
0x20434;
					}
					if prefixes & 2 != 0 {
						return  /* subss */ 
0x20412;
					} /* subps */
					0x20440
				}
				0x5e => {
					if prefixes & 8 != 0 {
						return  /* divpd */ 
0x20448;
					}
					if prefixes & 4 != 0 {
						return  /* divsd */ 
0x20434;
					}
					if prefixes & 2 != 0 {
						return  /* divss */ 
0x20412;
					} /* divps */
					0x20440
				}
				0x6c => {
					// Multiple prefixes
					// punpcklqdq

					0x20448
				}
				0x6d => {
					// Multiple prefixes
					// punpckhqdq

					0x20448
				}
				0x6e => {
					// Multiple prefixes
					// mov

					0x20418
				}
				0x6f => {
					if prefixes & 8 != 0 {
						return  /* movdqa */ 
0x20448;
					}
					if prefixes & 2 != 0 {
						return  /* movdqu */ 
0x20442;
					}
					0
				}
				0x70 => {
					// Multiple prefixes
					// pshufd

					0x204c8
				}
				0x7e => {
					if prefixes & 8 != 0 {
						return  /* mov */ 
0x20218;
					}
					if prefixes & 2 != 0 {
						return  /* movq */ 
0x20432;
					}
					0
				}
				0x7f => {
					if prefixes & 8 != 0 {
						return  /* movdqa */ 
0x20248;
					}
					if prefixes & 2 != 0 {
						return  /* movdqu */ 
0x20242;
					}
					0
				}
				0x80 => {
					/* jo */

					0x2a10
				}
				0x81 => {
					/* jno */

					0x2a10
				}
				0x82 => {
					/* jb */

					0x2a10
				}
				0x83 => {
					/* jae */

					0x2a10
				}
				0x84 => {
					/* je */

					0x2a10
				}
				0x85 => {
					/* jne */

					0x2a10
				}
				0x86 => {
					/* jbe */

					0x2a10
				}
				0x87 => {
					/* ja */

					0x2a10
				}
				0x88 => {
					/* js */

					0x2a10
				}
				0x89 => {
					/* jns */

					0x2a10
				}
				0x8a => {
					/* jp */

					0x2a10
				}
				0x8b => {
					/* jnp */

					0x2a10
				}
				0x8c => {
					/* jl */

					0x2a10
				}
				0x8d => {
					/* jge */

					0x2a10
				}
				0x8e => {
					/* jle */

					0x2a10
				}
				0x8f => {
					/* jg */

					0x2a10
				}
				0x90 => {
					/* seto */

					0x20200
				}
				0x91 => {
					/* setno */

					0x20200
				}
				0x92 => {
					/* setb */

					0x20200
				}
				0x93 => {
					/* setae */

					0x20200
				}
				0x94 => {
					/* sete */

					0x20200
				}
				0x95 => {
					/* setne */

					0x20200
				}
				0x96 => {
					/* setbe */

					0x20200
				}
				0x97 => {
					/* seta */

					0x20200
				}
				0x98 => {
					/* sets */

					0x20200
				}
				0x99 => {
					/* setns */

					0x20200
				}
				0x9a => {
					/* setp */

					0x20200
				}
				0x9b => {
					/* setnp */

					0x20200
				}
				0x9c => {
					/* setl */

					0x20200
				}
				0x9d => {
					/* setge */

					0x20200
				}
				0x9e => {
					/* setle */

					0x20200
				}
				0x9f => {
					/* setg */

					0x20200
				}
				0xa3 => {
					/* bt */

					0x20210
				}
				0xab => {
					/* bts */

					0x20211
				}
				0xae => {
					match c.next() {
						0xf0 => {
							/* mfence */

							0x2410
						}
						_ => 0,
					}
				}
				0xaf => {
					/* imul */

					0x20410
				}
				0xb0 => {
					/* cmpxchg */

					0x20201
				}
				0xb1 => {
					/* cmpxchg */

					0x20211
				}
				0xb3 => {
					/* btr */

					0x20211
				}
				0xb6 => {
					/* movzx */

					0x20400
				}
				0xb7 => {
					/* movzx */

					0x20420
				}
				0xba => {
					match (c.peek() >> 3u8) & 7 { 
						// 0 => capstone: unknown
						// 1 => capstone: unknown
						// 2 => capstone: unknown
						// 3 => capstone: unknown
						0x4 => {
							/* bt */

							0x20290
						}
						0x5 => {
							/* bts */

							0x20291
						}
						0x6 => {
							/* btr */

							0x20291
						}
						0x7 => {
							/* btc */

							0x20291
						}
						_ => 0,
					}
				}
				0xbb => {
					/* btc */

					0x20211
				}
				0xbe => {
					/* movsx */

					0x20400
				}
				0xbf => {
					/* movsx */

					0x20420
				}
				0xc0 => {
					/* xadd */

					0x20201
				}
				0xc1 => {
					/* xadd */

					0x20211
				}
				0xd6 => {
					// Multiple prefixes
					// movq

					0x20238
				}
				_ => 0,
			}
		}
		0x10 => {
			/* adc */

			0x20201
		}
		0x11 => {
			/* adc */

			0x20219
		}
		0x12 => {
			/* adc */

			0x20401
		}
		0x13 => {
			/* adc */

			0x20419
		}
		0x14 => {
			/* adc */

			0x42481
		}
		0x15 => {
			/* adc */

			0x42519
		}
		0x18 => {
			/* sbb */

			0x20201
		}
		0x19 => {
			/* sbb */

			0x20219
		}
		0x1a => {
			/* sbb */

			0x20401
		}
		0x1b => {
			/* sbb */

			0x20419
		}
		0x1c => {
			/* sbb */

			0x42481
		}
		0x1d => {
			/* sbb */

			0x42519
		}
		0x20 => {
			/* and */

			0x20201
		}
		0x21 => {
			/* and */

			0x20c19
		}
		0x22 => {
			/* and */

			0x20401
		}
		0x23 => {
			/* and */

			0x20e19
		}
		0x24 => {
			/* and */

			0x42481
		}
		0x25 => {
			/* and */

			0x42519
		}
		0x28 => {
			/* sub */

			0x20201
		}
		0x29 => {
			/* sub */

			0x20219
		}
		0x2a => {
			/* sub */

			0x20401
		}
		0x2b => {
			/* sub */

			0x20419
		}
		0x2c => {
			/* sub */

			0x42481
		}
		0x2d => {
			/* sub */

			0x42519
		}
		0x2e => {
			match c.next() {
				0xf => {
					match c.next() {
						0x1f => {
							match (c.peek() >> 3u8) & 7 { 
								0x0 => {
									/* nop */

									0x20218
								}
								// 1 => capstone: nop dword ptr cs:[rdx]
								// 2 => capstone: nop dword ptr cs:[rdx]
								// 3 => capstone: nop dword ptr cs:[rdx]
								// 4 => capstone: nop dword ptr cs:[rdx]
								// 5 => capstone: nop dword ptr cs:[rdx]
								// 6 => capstone: nop dword ptr cs:[rdx]
								// 7 => capstone: nop dword ptr cs:[rdx]
								_ => 0,
							}
						}
						_ => 0,
					}
				}
				_ => 0,
			}
		}
		0x30 => {
			/* xor */

			0x20201
		}
		0x31 => {
			/* xor */

			0x20219
		}
		0x32 => {
			/* xor */

			0x20401
		}
		0x33 => {
			/* xor */

			0x20419
		}
		0x34 => {
			/* xor */

			0x42481
		}
		0x35 => {
			/* xor */

			0x42519
		}
		0x38 => {
			/* cmp */

			0x20600
		}
		0x39 => {
			/* cmp */

			0x20618
		}
		0x3a => {
			/* cmp */

			0x20600
		}
		0x3b => {
			/* cmp */

			0x20618
		}
		0x3c => {
			/* cmp */

			0x2480
		}
		0x3d => {
			/* cmp */

			0x2518
		}
		0x50 => {
			/* push */

			0x1230
		}
		0x51 => {
			/* push */

			0x1230
		}
		0x52 => {
			/* push */

			0x1230
		}
		0x53 => {
			/* push */

			0x1230
		}
		0x54 => {
			/* push */

			0x1230
		}
		0x55 => {
			/* push */

			0x1230
		}
		0x56 => {
			/* push */

			0x1230
		}
		0x57 => {
			/* push */

			0x1230
		}
		0x58 => {
			/* pop */

			0x1430
		}
		0x59 => {
			/* pop */

			0x1430
		}
		0x5a => {
			/* pop */

			0x1430
		}
		0x5b => {
			/* pop */

			0x1430
		}
		0x5c => {
			/* pop */

			0x1430
		}
		0x5d => {
			/* pop */

			0x1430
		}
		0x5e => {
			/* pop */

			0x1430
		}
		0x5f => {
			/* pop */

			0x1430
		}
		0x63 => {
			/* movsxd */

			0x20410
		}
		0x66 => {
			match c.next() {
				0x2e => {
					match c.next() {
						0xf => {
							match c.next() {
								0x1f => {
									match (c.peek() >> 3u8) & 7 { 
										0x0 => {
											/* nop */

											0x20218
										}
										// 1 => capstone: nop word ptr cs:[rdx]
										// 2 => capstone: nop word ptr cs:[rdx]
										// 3 => capstone: nop word ptr cs:[rdx]
										// 4 => capstone: nop word ptr cs:[rdx]
										// 5 => capstone: nop word ptr cs:[rdx]
										// 6 => capstone: nop word ptr cs:[rdx]
										// 7 => capstone: nop word ptr cs:[rdx]
										_ => 0,
									}
								}
								_ => 0,
							}
						}
						_ => 0,
					}
				}
				0x66 => {
					match c.next() {
						0x2e => {
							match c.next() {
								0xf => {
									match c.next() {
										0x1f => {
											match (c.peek() >> 3u8) & 7 { 
												0x0 => {
													/* nop */

													0x20218
												}
												// 1 => capstone: nop word ptr cs:[rdx]
												// 2 => capstone: nop word ptr cs:[rdx]
												// 3 => capstone: nop word ptr cs:[rdx]
												// 4 => capstone: nop word ptr cs:[rdx]
												// 5 => capstone: nop word ptr cs:[rdx]
												// 6 => capstone: nop word ptr cs:[rdx]
												// 7 => capstone: nop word ptr cs:[rdx]
												_ => 0,
											}
										}
										_ => 0,
									}
								}
								_ => 0,
							}
						}
						0x66 => {
							match c.next() {
								0x2e => {
									match c.next() {
										0xf => {
											match c.next() {
												0x1f => {
													match (c.peek() >> 3u8) & 7 { 
														0x0 => {
															/* nop */

															0x20218
														}
														// 1 => capstone: nop word ptr cs:[rdx]
														// 2 => capstone: nop word ptr cs:[rdx]
														// 3 => capstone: nop word ptr cs:[rdx]
														// 4 => capstone: nop word ptr cs:[rdx]
														// 5 => capstone: nop word ptr cs:[rdx]
														// 6 => capstone: nop word ptr cs:[rdx]
														// 7 => capstone: nop word ptr cs:[rdx]
														_ => 0,
													}
												}
												_ => 0,
											}
										}
										_ => 0,
									}
								}
								0x66 => {
									match c.next() {
										0x2e => {
											match c.next() {
												0xf => {
													match c.next() {
														0x1f => {
															match (c.peek() >> 3u8) & 7 { 
																0x0 => {
																	/* nop */

																	0x20218
																}
																// 1 => capstone: nop word ptr cs:[rdx]
																// 2 => capstone: nop word ptr cs:[rdx]
																// 3 => capstone: nop word ptr cs:[rdx]
																// 4 => capstone: nop word ptr cs:[rdx]
																// 5 => capstone: nop word ptr cs:[rdx]
																// 6 => capstone: nop word ptr cs:[rdx]
																// 7 => capstone: nop word ptr cs:[rdx]
																_ => 0,
															}
														}
														_ => 0,
													}
												}
												_ => 0,
											}
										}
										_ => 0,
									}
								}
								_ => 0,
							}
						}
						_ => 0,
					}
				}
				_ => 0,
			}
		}
		0x69 => {
			/* imul */

			0x20510
		}
		0x6b => {
			/* imul */

			0x20490
		}
		0x70 => {
			/* jo */

			0x2810
		}
		0x71 => {
			/* jno */

			0x2810
		}
		0x72 => {
			/* jb */

			0x2810
		}
		0x73 => {
			/* jae */

			0x2810
		}
		0x74 => {
			/* je */

			0x2810
		}
		0x75 => {
			/* jne */

			0x2810
		}
		0x76 => {
			/* jbe */

			0x2810
		}
		0x77 => {
			/* ja */

			0x2810
		}
		0x78 => {
			/* js */

			0x2810
		}
		0x79 => {
			/* jns */

			0x2810
		}
		0x7a => {
			/* jp */

			0x2810
		}
		0x7b => {
			/* jnp */

			0x2810
		}
		0x7c => {
			/* jl */

			0x2810
		}
		0x7d => {
			/* jge */

			0x2810
		}
		0x7e => {
			/* jle */

			0x2810
		}
		0x7f => {
			/* jg */

			0x2810
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
			0x20280
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
			0x20318
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
			0x20298
		}
		0x84 => {
			/* test */

			0x20600
		}
		0x85 => {
			/* test */

			0x20618
		}
		0x86 => {
			/* xchg */

			0x20200
		}
		0x87 => {
			/* xchg */

			0x20210
		}
		0x88 => {
			/* mov */

			0x20200
		}
		0x89 => {
			/* mov */

			0x20818
		}
		0x8a => {
			/* mov */

			0x20400
		}
		0x8b => {
			/* mov */

			0x20a18
		}
		0x8d => {
			/* lea */

			0x21010
		}
		0x90 => {
			if prefixes & 2 != 0 {
				return  /* pause */ 
0x2412;
			} /* nop */
			0x2418
		}
		0x91 => {
			/* xchg */

			0x42410
		}
		0x92 => {
			/* xchg */

			0x42410
		}
		0x93 => {
			/* xchg */

			0x42410
		}
		0x94 => {
			/* xchg */

			0x42410
		}
		0x95 => {
			/* xchg */

			0x42410
		}
		0x96 => {
			/* xchg */

			0x42410
		}
		0x97 => {
			/* xchg */

			0x42410
		}
		0x98 => {
			if prefixes & 8 != 0 {
				return  /* cbw */ 
0x42418;
			} /* cwde */
			0x42410
		}
		0x99 => {
			if prefixes & 8 != 0 {
				return  /* cwd */ 
0x82418;
			} /* cdq */
			0x82410
		}
		0xa0 => {
			/* mov */

			0x41800
		}
		0xa1 => {
			/* mov */

			0x41810
		}
		0xa2 => {
			/* mov */

			0x1800
		}
		0xa3 => {
			/* mov */

			0x1810
		}
		0xa4 => {
			/* movs */

			0x2400
		}
		0xa5 => {
			/* movs */

			0x2410
		}
		0xa8 => {
			/* test */

			0x2480
		}
		0xa9 => {
			/* test */

			0x2518
		}
		0xb0 => {
			/* mov */

			0x1680
		}
		0xb1 => {
			/* mov */

			0x1680
		}
		0xb2 => {
			/* mov */

			0x1680
		}
		0xb3 => {
			/* mov */

			0x1680
		}
		0xb4 => {
			/* mov */

			0x1680
		}
		0xb5 => {
			/* mov */

			0x1680
		}
		0xb6 => {
			/* mov */

			0x1680
		}
		0xb7 => {
			/* mov */

			0x1680
		}
		0xb8 => {
			/* mov */

			0x1798
		}
		0xb9 => {
			/* mov */

			0x1798
		}
		0xba => {
			/* mov */

			0x1798
		}
		0xbb => {
			/* mov */

			0x1798
		}
		0xbc => {
			/* mov */

			0x1798
		}
		0xbd => {
			/* mov */

			0x1798
		}
		0xbe => {
			/* mov */

			0x1798
		}
		0xbf => {
			/* mov */

			0x1798
		}
		0xc0 => {
			match (c.peek() >> 3u8) & 7 { 
				0x0 => {
					/* rol */

					0x20280
				}
				0x1 => {
					/* ror */

					0x20280
				}
				0x2 => {
					/* rcl */

					0x20280
				}
				0x3 => {
					/* rcr */

					0x20280
				}
				0x4 => {
					/* shl */

					0x20280
				}
				0x5 => {
					/* shr */

					0x20280
				}
				// 6 => capstone: rcr byte ptr [rdx], 0x1a
				0x7 => {
					/* sar */

					0x20280
				}
				_ => 0,
			}
		}
		0xc1 => {
			match (c.peek() >> 3u8) & 7 { 
				0x0 => {
					/* rol */

					0x20290
				}
				0x1 => {
					/* ror */

					0x20290
				}
				0x2 => {
					/* rcl */

					0x20290
				}
				0x3 => {
					/* rcr */

					0x20290
				}
				0x4 => {
					/* shl */

					0x20290
				}
				0x5 => {
					/* shr */

					0x20290
				}
				// 6 => capstone: rcr dword ptr [rdx], 0x1a
				0x7 => {
					/* sar */

					0x20290
				}
				_ => 0,
			}
		}
		0xc3 => {
			/* ret */

			0x2610
		}
		0xc6 => {
			match (c.peek() >> 3u8) & 7 { 
				0x0 => {
					/* mov */

					0x20280
				}
				// 1 => capstone: unknown
				// 2 => capstone: unknown
				// 3 => capstone: unknown
				// 4 => capstone: unknown
				// 5 => capstone: unknown
				// 6 => capstone: unknown
				// 7 => capstone: unknown
				_ => 0,
			}
		}
		0xc7 => {
			match (c.peek() >> 3u8) & 7 { 
				0x0 => {
					/* mov */

					0x20318
				}
				// 1 => capstone: unknown
				// 2 => capstone: unknown
				// 3 => capstone: unknown
				// 4 => capstone: unknown
				// 5 => capstone: unknown
				// 6 => capstone: unknown
				// 7 => capstone: unknown
				_ => 0,
			}
		}
		0xcc => {
			/* int3 */

			0x2410
		}
		0xcd => {
			/* int */

			0x2480
		}
		0xd0 => {
			match (c.peek() >> 3u8) & 7 { 
				0x0 => {
					/* rol */

					0x20200
				}
				0x1 => {
					/* ror */

					0x20200
				}
				0x2 => {
					/* rcl */

					0x20200
				}
				0x3 => {
					/* rcr */

					0x20200
				}
				0x4 => {
					/* shl */

					0x20200
				}
				0x5 => {
					/* shr */

					0x20200
				}
				// 6 => capstone: rcr byte ptr [rdx], 1
				0x7 => {
					/* sar */

					0x20200
				}
				_ => 0,
			}
		}
		0xd1 => {
			match (c.peek() >> 3u8) & 7 { 
				0x0 => {
					/* rol */

					0x20210
				}
				0x1 => {
					/* ror */

					0x20210
				}
				0x2 => {
					/* rcl */

					0x20210
				}
				0x3 => {
					/* rcr */

					0x20210
				}
				0x4 => {
					/* shl */

					0x20210
				}
				0x5 => {
					/* shr */

					0x20210
				}
				// 6 => capstone: rcr dword ptr [rdx], 1
				0x7 => {
					/* sar */

					0x20210
				}
				_ => 0,
			}
		}
		0xd2 => {
			match (c.peek() >> 3u8) & 7 { 
				0x0 => {
					/* rol */

					0x20200
				}
				0x1 => {
					/* ror */

					0x20200
				}
				0x2 => {
					/* rcl */

					0x20200
				}
				0x3 => {
					/* rcr */

					0x20200
				}
				0x4 => {
					/* shl */

					0x20200
				}
				0x5 => {
					/* shr */

					0x20200
				}
				// 6 => capstone: rcr byte ptr [rdx], cl
				0x7 => {
					/* sar */

					0x20200
				}
				_ => 0,
			}
		}
		0xd3 => {
			match (c.peek() >> 3u8) & 7 { 
				0x0 => {
					/* rol */

					0x20210
				}
				0x1 => {
					/* ror */

					0x20210
				}
				0x2 => {
					/* rcl */

					0x20210
				}
				0x3 => {
					/* rcr */

					0x20210
				}
				0x4 => {
					/* shl */

					0x20210
				}
				0x5 => {
					/* shr */

					0x20210
				}
				// 6 => capstone: rcr dword ptr [rdx], cl
				0x7 => {
					/* sar */

					0x20210
				}
				_ => 0,
			}
		}
		0xe8 => {
			/* call */

			0x1c10
		}
		0xe9 => {
			/* jmp */

			0x1e10
		}
		0xeb => {
			/* jmp */

			0x2010
		}
		0xf6 => {
			match (c.peek() >> 3u8) & 7 { 
				0x0 => {
					/* test */

					0x20680
				}
				// 1 => capstone: neg byte ptr [rdx]
				0x2 => {
					/* not */

					0x20201
				}
				0x3 => {
					/* neg */

					0x20201
				}
				0x4 => {
					/* mul */

					0x60600
				}
				0x5 => {
					/* imul */

					0x60600
				}
				0x6 => {
					/* div */

					0x60600
				}
				0x7 => {
					/* idiv */

					0x60600
				}
				_ => 0,
			}
		}
		0xf7 => {
			match (c.peek() >> 3u8) & 7 { 
				0x0 => {
					/* test */

					0x20718
				}
				// 1 => capstone: neg dword ptr [rdx]
				0x2 => {
					/* not */

					0x20211
				}
				0x3 => {
					/* neg */

					0x20211
				}
				0x4 => {
					/* mul */

					0xe0610
				}
				0x5 => {
					/* imul */

					0xe0610
				}
				0x6 => {
					/* div */

					0xe0610
				}
				0x7 => {
					/* idiv */

					0xe0610
				}
				_ => 0,
			}
		}
		0xfe => {
			match (c.peek() >> 3u8) & 7 { 
				0x0 => {
					/* inc */

					0x20201
				}
				0x1 => {
					/* dec */

					0x20201
				}
				// 2 => capstone: unknown
				// 3 => capstone: unknown
				// 4 => capstone: unknown
				// 5 => capstone: unknown
				// 6 => capstone: unknown
				// 7 => capstone: unknown
				_ => 0,
			}
		}
		0xff => {
			match (c.peek() >> 3u8) & 7 { 
				0x0 => {
					/* inc */

					0x20211
				}
				0x1 => {
					/* dec */

					0x20211
				}
				0x2 => {
					/* call */

					0x20210
				}
				// 3 => capstone: lcall ptr [rdx]
				0x4 => {
					/* jmp */

					0x20210
				}
				// 5 => capstone: lcall ptr [rdx]
				// 6 => capstone: lcall ptr [rdx]
				// 7 => capstone: lcall ptr [rdx]
				_ => 0,
			}
		}
		_ => 0,
	}
}
