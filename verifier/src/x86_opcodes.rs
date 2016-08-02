use x86_decoder::{Cursor, CursorError};
pub fn decode(c: &mut Cursor, prefixes: u32) -> Result<u32, CursorError> {
	Ok(match try!(c.next()) {
		0x0 => {
			/* add */

			0x20081
		}
		0x1 => {
			/* add */

			0x20099
		}
		0x2 => {
			/* add */

			0x20101
		}
		0x3 => {
			/* add */

			0x20119
		}
		0x4 => {
			/* add */

			0x48901
		}
		0x5 => {
			/* add */

			0x50919
		}
		0x8 => {
			/* or */

			0x20081
		}
		0x9 => {
			/* or */

			0x20099
		}
		0xa => {
			/* or */

			0x20101
		}
		0xb => {
			/* or */

			0x20119
		}
		0xc => {
			/* or */

			0x48901
		}
		0xd => {
			/* or */

			0x50919
		}
		0xf => {
			match try!(c.next()) {
				0xb => {
					/* ud2 */

					0x890
				}
				0x10 => {
					if prefixes & 8 != 0 {
						return Ok(/* movupd */
						          0x20148);
					}
					if prefixes & 4 != 0 {
						return Ok(/* movsd */
						          0x20134);
					} /* movups */
					0x20140
				}
				0x11 => {
					if prefixes & 8 != 0 {
						return Ok(/* movupd */
						          0x200c8);
					}
					if prefixes & 4 != 0 {
						return Ok(/* movsd */
						          0x200b4);
					} /* movups */
					0x200c0
				}
				0x1f => {
					match (try!(c.peek()) >> 3u8) & 7 { 
						0x0 => {
							/* nop */

							0x20c98
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
						return Ok(/* movapd */
						          0x20148);
					} /* movaps */
					0x20140
				}
				0x29 => {
					if prefixes & 8 != 0 {
						return Ok(/* movapd */
						          0x200c8);
					} /* movaps */
					0x200c0
				}
				0x2a => {
					if prefixes & 4 != 0 {
						return Ok(/* cvtsi2sd */
						          0x20114);
					}
					if prefixes & 2 != 0 {
						return Ok(/* cvtsi2ss */
						          0x20112);
					}
					0
				}
				0x2c => {
					if prefixes & 4 != 0 {
						return Ok(/* cvttsd2si */
						          0x20134);
					}
					if prefixes & 2 != 0 {
						return Ok(/* cvttss2si */
						          0x20112);
					}
					0
				}
				0x2e => {
					if prefixes & 8 != 0 {
						return Ok(/* ucomisd */
						          0x201b8);
					} /* ucomiss */
					0x20190
				}
				0x38 => {
					match try!(c.next()) {
						0x0 => {
							// Multiple prefixes
							// pshufb

							0x20148
						}
						_ => 0,
					}
				}
				0x40 => {
					/* cmovo */

					0x20110
				}
				0x41 => {
					/* cmovno */

					0x20110
				}
				0x42 => {
					/* cmovb */

					0x20110
				}
				0x43 => {
					/* cmovae */

					0x20110
				}
				0x44 => {
					/* cmove */

					0x20110
				}
				0x45 => {
					/* cmovne */

					0x20110
				}
				0x46 => {
					/* cmovbe */

					0x20110
				}
				0x47 => {
					/* cmova */

					0x20110
				}
				0x48 => {
					/* cmovs */

					0x20110
				}
				0x49 => {
					/* cmovns */

					0x20110
				}
				0x4a => {
					/* cmovp */

					0x20110
				}
				0x4b => {
					/* cmovnp */

					0x20110
				}
				0x4c => {
					/* cmovl */

					0x20110
				}
				0x4d => {
					/* cmovge */

					0x20110
				}
				0x4e => {
					/* cmovle */

					0x20110
				}
				0x4f => {
					/* cmovg */

					0x20110
				}
				0x54 => {
					if prefixes & 8 != 0 {
						return Ok(/* andpd */
						          0x20148);
					} /* andps */
					0x20140
				}
				0x57 => {
					if prefixes & 8 != 0 {
						return Ok(/* xorpd */
						          0x20148);
					} /* xorps */
					0x20140
				}
				0x58 => {
					if prefixes & 8 != 0 {
						return Ok(/* addpd */
						          0x20148);
					}
					if prefixes & 4 != 0 {
						return Ok(/* addsd */
						          0x20134);
					}
					if prefixes & 2 != 0 {
						return Ok(/* addss */
						          0x20112);
					} /* addps */
					0x20140
				}
				0x59 => {
					if prefixes & 8 != 0 {
						return Ok(/* mulpd */
						          0x20148);
					}
					if prefixes & 4 != 0 {
						return Ok(/* mulsd */
						          0x20134);
					}
					if prefixes & 2 != 0 {
						return Ok(/* mulss */
						          0x20112);
					} /* mulps */
					0x20140
				}
				0x5c => {
					if prefixes & 8 != 0 {
						return Ok(/* subpd */
						          0x20148);
					}
					if prefixes & 4 != 0 {
						return Ok(/* subsd */
						          0x20134);
					}
					if prefixes & 2 != 0 {
						return Ok(/* subss */
						          0x20112);
					} /* subps */
					0x20140
				}
				0x5e => {
					if prefixes & 8 != 0 {
						return Ok(/* divpd */
						          0x20148);
					}
					if prefixes & 4 != 0 {
						return Ok(/* divsd */
						          0x20134);
					}
					if prefixes & 2 != 0 {
						return Ok(/* divss */
						          0x20112);
					} /* divps */
					0x20140
				}
				0x6c => {
					// Multiple prefixes
					// punpcklqdq

					0x20148
				}
				0x6d => {
					// Multiple prefixes
					// punpckhqdq

					0x20148
				}
				0x6e => {
					// Multiple prefixes
					// mov

					0x20118
				}
				0x6f => {
					if prefixes & 8 != 0 {
						return Ok(/* movdqa */
						          0x20148);
					}
					if prefixes & 2 != 0 {
						return Ok(/* movdqu */
						          0x20142);
					}
					0
				}
				0x70 => {
					// Multiple prefixes
					// pshufd

					0x28148
				}
				0x7e => {
					if prefixes & 8 != 0 {
						return Ok(/* mov */
						          0x20098);
					}
					if prefixes & 2 != 0 {
						return Ok(/* movq */
						          0x20132);
					}
					0
				}
				0x7f => {
					if prefixes & 8 != 0 {
						return Ok(/* movdqa */
						          0x200c8);
					}
					if prefixes & 2 != 0 {
						return Ok(/* movdqu */
						          0x200c2);
					}
					0
				}
				0x80 => {
					/* jo */

					0xa90
				}
				0x81 => {
					/* jno */

					0xa90
				}
				0x82 => {
					/* jb */

					0xa90
				}
				0x83 => {
					/* jae */

					0xa90
				}
				0x84 => {
					/* je */

					0xa90
				}
				0x85 => {
					/* jne */

					0xa90
				}
				0x86 => {
					/* jbe */

					0xa90
				}
				0x87 => {
					/* ja */

					0xa90
				}
				0x88 => {
					/* js */

					0xa90
				}
				0x89 => {
					/* jns */

					0xa90
				}
				0x8a => {
					/* jp */

					0xa90
				}
				0x8b => {
					/* jnp */

					0xa90
				}
				0x8c => {
					/* jl */

					0xa90
				}
				0x8d => {
					/* jge */

					0xa90
				}
				0x8e => {
					/* jle */

					0xa90
				}
				0x8f => {
					/* jg */

					0xa90
				}
				0x90 => {
					/* seto */

					0x20080
				}
				0x91 => {
					/* setno */

					0x20080
				}
				0x92 => {
					/* setb */

					0x20080
				}
				0x93 => {
					/* setae */

					0x20080
				}
				0x94 => {
					/* sete */

					0x20080
				}
				0x95 => {
					/* setne */

					0x20080
				}
				0x96 => {
					/* setbe */

					0x20080
				}
				0x97 => {
					/* seta */

					0x20080
				}
				0x98 => {
					/* sets */

					0x20080
				}
				0x99 => {
					/* setns */

					0x20080
				}
				0x9a => {
					/* setp */

					0x20080
				}
				0x9b => {
					/* setnp */

					0x20080
				}
				0x9c => {
					/* setl */

					0x20080
				}
				0x9d => {
					/* setge */

					0x20080
				}
				0x9e => {
					/* setle */

					0x20080
				}
				0x9f => {
					/* setg */

					0x20080
				}
				0xa3 => {
					/* bt */

					0x20090
				}
				0xab => {
					/* bts */

					0x20091
				}
				0xae => {
					match try!(c.next()) {
						0xf0 => {
							/* mfence */

							0x910
						}
						_ => 0,
					}
				}
				0xaf => {
					/* imul */

					0x20110
				}
				0xb0 => {
					/* cmpxchg */

					0x20081
				}
				0xb1 => {
					/* cmpxchg */

					0x20091
				}
				0xb3 => {
					/* btr */

					0x20091
				}
				0xb6 => {
					/* movzx */

					0x20100
				}
				0xb7 => {
					/* movzx */

					0x20120
				}
				0xba => {
					match (try!(c.peek()) >> 3u8) & 7 { 
						// 0 => capstone: unknown
						// 1 => capstone: unknown
						// 2 => capstone: unknown
						// 3 => capstone: unknown
						0x4 => {
							/* bt */

							0x28090
						}
						0x5 => {
							/* bts */

							0x28091
						}
						0x6 => {
							/* btr */

							0x28091
						}
						0x7 => {
							/* btc */

							0x28091
						}
						_ => 0,
					}
				}
				0xbb => {
					/* btc */

					0x20091
				}
				0xbc => {
					/* bsf */

					0x20111
				}
				0xbd => {
					/* bsr */

					0x20111
				}
				0xbe => {
					/* movsx */

					0x20100
				}
				0xbf => {
					/* movsx */

					0x20120
				}
				0xc0 => {
					/* xadd */

					0x20081
				}
				0xc1 => {
					/* xadd */

					0x20091
				}
				0xd6 => {
					// Multiple prefixes
					// movq

					0x200b8
				}
				_ => 0,
			}
		}
		0x10 => {
			/* adc */

			0x20081
		}
		0x11 => {
			/* adc */

			0x20099
		}
		0x12 => {
			/* adc */

			0x20101
		}
		0x13 => {
			/* adc */

			0x20119
		}
		0x14 => {
			/* adc */

			0x48901
		}
		0x15 => {
			/* adc */

			0x50919
		}
		0x18 => {
			/* sbb */

			0x20081
		}
		0x19 => {
			/* sbb */

			0x20099
		}
		0x1a => {
			/* sbb */

			0x20101
		}
		0x1b => {
			/* sbb */

			0x20119
		}
		0x1c => {
			/* sbb */

			0x48901
		}
		0x1d => {
			/* sbb */

			0x50919
		}
		0x20 => {
			/* and */

			0x20081
		}
		0x21 => {
			/* and */

			0x20319
		}
		0x22 => {
			/* and */

			0x20101
		}
		0x23 => {
			/* and */

			0x20399
		}
		0x24 => {
			/* and */

			0x48901
		}
		0x25 => {
			/* and */

			0x50919
		}
		0x28 => {
			/* sub */

			0x20081
		}
		0x29 => {
			/* sub */

			0x20099
		}
		0x2a => {
			/* sub */

			0x20101
		}
		0x2b => {
			/* sub */

			0x20119
		}
		0x2c => {
			/* sub */

			0x48901
		}
		0x2d => {
			/* sub */

			0x50919
		}
		0x2e => {
			match try!(c.next()) {
				0xf => {
					match try!(c.next()) {
						0x1f => {
							match (try!(c.peek()) >> 3u8) & 7 { 
								0x0 => {
									/* nop */

									0x20c98
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

			0x20081
		}
		0x31 => {
			/* xor */

			0x20099
		}
		0x32 => {
			/* xor */

			0x20101
		}
		0x33 => {
			/* xor */

			0x20119
		}
		0x34 => {
			/* xor */

			0x48901
		}
		0x35 => {
			/* xor */

			0x50919
		}
		0x38 => {
			/* cmp */

			0x20180
		}
		0x39 => {
			/* cmp */

			0x20198
		}
		0x3a => {
			/* cmp */

			0x20180
		}
		0x3b => {
			/* cmp */

			0x20198
		}
		0x3c => {
			/* cmp */

			0x8900
		}
		0x3d => {
			/* cmp */

			0x10918
		}
		0x50 => {
			/* push */

			0x4b0
		}
		0x51 => {
			/* push */

			0x4b0
		}
		0x52 => {
			/* push */

			0x4b0
		}
		0x53 => {
			/* push */

			0x4b0
		}
		0x54 => {
			/* push */

			0x4b0
		}
		0x55 => {
			/* push */

			0x4b0
		}
		0x56 => {
			/* push */

			0x4b0
		}
		0x57 => {
			/* push */

			0x4b0
		}
		0x58 => {
			/* pop */

			0x530
		}
		0x59 => {
			/* pop */

			0x530
		}
		0x5a => {
			/* pop */

			0x530
		}
		0x5b => {
			/* pop */

			0x530
		}
		0x5c => {
			/* pop */

			0x530
		}
		0x5d => {
			/* pop */

			0x530
		}
		0x5e => {
			/* pop */

			0x530
		}
		0x5f => {
			/* pop */

			0x530
		}
		0x63 => {
			/* movsxd */

			0x20110
		}
		0x66 => {
			match try!(c.next()) {
				0x2e => {
					match try!(c.next()) {
						0xf => {
							match try!(c.next()) {
								0x1f => {
									match (try!(c.peek()) >> 3u8) & 7 { 
										0x0 => {
											/* nop */

											0x20c98
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
					match try!(c.next()) {
						0x2e => {
							match try!(c.next()) {
								0xf => {
									match try!(c.next()) {
										0x1f => {
											match (try!(c.peek()) >> 3u8) & 7 { 
												0x0 => {
													/* nop */

													0x20c98
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
							match try!(c.next()) {
								0x2e => {
									match try!(c.next()) {
										0xf => {
											match try!(c.next()) {
												0x1f => {
													match (try!(c.peek()) >> 3u8) & 7 { 
														0x0 => {
															/* nop */

															0x20c98
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
									match try!(c.next()) {
										0x2e => {
											match try!(c.next()) {
												0xf => {
													match try!(c.next()) {
														0x1f => {
															match (try!(c.peek()) >> 3u8) & 7 { 
																0x0 => {
																	/* nop */

																	0x20c98
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
											match try!(c.next()) {
												0x2e => {
													match try!(c.next()) {
														0xf => {
															match try!(c.next()) {
																0x1f => {
																	match (try!(c.peek()) >> 3u8) & 7 { 
																		0x0 => {
																			/* nop */

																			0x20c98
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
				_ => 0,
			}
		}
		0x69 => {
			/* imul */

			0x30110
		}
		0x6b => {
			/* imul */

			0x28110
		}
		0x70 => {
			/* jo */

			0xa10
		}
		0x71 => {
			/* jno */

			0xa10
		}
		0x72 => {
			/* jb */

			0xa10
		}
		0x73 => {
			/* jae */

			0xa10
		}
		0x74 => {
			/* je */

			0xa10
		}
		0x75 => {
			/* jne */

			0xa10
		}
		0x76 => {
			/* jbe */

			0xa10
		}
		0x77 => {
			/* ja */

			0xa10
		}
		0x78 => {
			/* js */

			0xa10
		}
		0x79 => {
			/* jns */

			0xa10
		}
		0x7a => {
			/* jp */

			0xa10
		}
		0x7b => {
			/* jnp */

			0xa10
		}
		0x7c => {
			/* jl */

			0xa10
		}
		0x7d => {
			/* jge */

			0xa10
		}
		0x7e => {
			/* jle */

			0xa10
		}
		0x7f => {
			/* jg */

			0xa10
		}
		0x80 => {
			match (try!(c.peek()) >> 3u8) & 7 { 
				0x0 => {
					/* add */

					0x28b81
				}
				0x1 => {
					/* or */

					0x28081
				}
				0x2 => {
					/* adc */

					0x28081
				}
				0x3 => {
					/* sbb */

					0x28081
				}
				0x4 => {
					/* and */

					0x28081
				}
				0x5 => {
					/* sub */

					0x28c01
				}
				0x6 => {
					/* xor */

					0x28081
				}
				0x7 => {
					/* cmp */

					0x28180
				}
				_ => 0,
			}
		}
		0x81 => {
			match (try!(c.peek()) >> 3u8) & 7 { 
				0x0 => {
					/* add */

					0x30b99
				}
				0x1 => {
					/* or */

					0x30099
				}
				0x2 => {
					/* adc */

					0x30099
				}
				0x3 => {
					/* sbb */

					0x30099
				}
				0x4 => {
					/* and */

					0x30099
				}
				0x5 => {
					/* sub */

					0x30c19
				}
				0x6 => {
					/* xor */

					0x30099
				}
				0x7 => {
					/* cmp */

					0x30198
				}
				_ => 0,
			}
		}
		0x83 => {
			match (try!(c.peek()) >> 3u8) & 7 { 
				0x0 => {
					/* add */

					0x28b99
				}
				0x1 => {
					/* or */

					0x28099
				}
				0x2 => {
					/* adc */

					0x28099
				}
				0x3 => {
					/* sbb */

					0x28099
				}
				0x4 => {
					/* and */

					0x28099
				}
				0x5 => {
					/* sub */

					0x28c19
				}
				0x6 => {
					/* xor */

					0x28099
				}
				0x7 => {
					/* cmp */

					0x28198
				}
				_ => 0,
			}
		}
		0x84 => {
			/* test */

			0x20180
		}
		0x85 => {
			/* test */

			0x20198
		}
		0x86 => {
			/* xchg */

			0x20b00
		}
		0x87 => {
			/* xchg */

			0x20b10
		}
		0x88 => {
			/* mov */

			0x20080
		}
		0x89 => {
			/* mov */

			0x20218
		}
		0x8a => {
			/* mov */

			0x20100
		}
		0x8b => {
			/* mov */

			0x20298
		}
		0x8d => {
			/* lea */

			0x20410
		}
		0x90 => {
			if prefixes & 2 != 0 {
				return Ok(/* pause */
				          0x912);
			} /* nop */
			0x918
		}
		0x91 => {
			/* xchg */

			0x40590
		}
		0x92 => {
			/* xchg */

			0x40590
		}
		0x93 => {
			/* xchg */

			0x40590
		}
		0x94 => {
			/* xchg */

			0x40590
		}
		0x95 => {
			/* xchg */

			0x40590
		}
		0x96 => {
			/* xchg */

			0x40590
		}
		0x97 => {
			/* xchg */

			0x40590
		}
		0x98 => {
			if prefixes & 8 != 0 {
				return Ok(/* cbw */
				          0x40918);
			} /* cwde */
			0x40910
		}
		0x99 => {
			if prefixes & 8 != 0 {
				return Ok(/* cwd */
				          0x80918);
			} /* cdq */
			0x80910
		}
		0xa0 => {
			/* mov */

			0x40600
		}
		0xa1 => {
			/* mov */

			0x40610
		}
		0xa2 => {
			/* mov */

			0x600
		}
		0xa3 => {
			/* mov */

			0x610
		}
		0xa4 => {
			/* movs */

			0x900
		}
		0xa5 => {
			/* movs */

			0x910
		}
		0xa8 => {
			/* test */

			0x8900
		}
		0xa9 => {
			/* test */

			0x10918
		}
		0xb0 => {
			/* mov */

			0x8580
		}
		0xb1 => {
			/* mov */

			0x8580
		}
		0xb2 => {
			/* mov */

			0x8580
		}
		0xb3 => {
			/* mov */

			0x8580
		}
		0xb4 => {
			/* mov */

			0x8580
		}
		0xb5 => {
			/* mov */

			0x8580
		}
		0xb6 => {
			/* mov */

			0x8580
		}
		0xb7 => {
			/* mov */

			0x8580
		}
		0xb8 => {
			/* mov */

			0x18598
		}
		0xb9 => {
			/* mov */

			0x18598
		}
		0xba => {
			/* mov */

			0x18598
		}
		0xbb => {
			/* mov */

			0x18598
		}
		0xbc => {
			/* mov */

			0x18598
		}
		0xbd => {
			/* mov */

			0x18598
		}
		0xbe => {
			/* mov */

			0x18598
		}
		0xbf => {
			/* mov */

			0x18598
		}
		0xc0 => {
			match (try!(c.peek()) >> 3u8) & 7 { 
				0x0 => {
					/* rol */

					0x28080
				}
				0x1 => {
					/* ror */

					0x28080
				}
				0x2 => {
					/* rcl */

					0x28080
				}
				0x3 => {
					/* rcr */

					0x28080
				}
				0x4 => {
					/* shl */

					0x28080
				}
				0x5 => {
					/* shr */

					0x28080
				}
				// 6 => capstone: rcr byte ptr [rdx], 0x1a
				0x7 => {
					/* sar */

					0x28080
				}
				_ => 0,
			}
		}
		0xc1 => {
			match (try!(c.peek()) >> 3u8) & 7 { 
				0x0 => {
					/* rol */

					0x28090
				}
				0x1 => {
					/* ror */

					0x28090
				}
				0x2 => {
					/* rcl */

					0x28090
				}
				0x3 => {
					/* rcr */

					0x28090
				}
				0x4 => {
					/* shl */

					0x28090
				}
				0x5 => {
					/* shr */

					0x28090
				}
				// 6 => capstone: rcr dword ptr [rdx], 0x1a
				0x7 => {
					/* sar */

					0x28090
				}
				_ => 0,
			}
		}
		0xc3 => {
			/* ret */

			0x990
		}
		0xc6 => {
			match (try!(c.peek()) >> 3u8) & 7 { 
				0x0 => {
					/* mov */

					0x28080
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
			match (try!(c.peek()) >> 3u8) & 7 { 
				0x0 => {
					/* mov */

					0x30098
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

			0x910
		}
		0xcd => {
			/* int */

			0x8900
		}
		0xd0 => {
			match (try!(c.peek()) >> 3u8) & 7 { 
				0x0 => {
					/* rol */

					0x20080
				}
				0x1 => {
					/* ror */

					0x20080
				}
				0x2 => {
					/* rcl */

					0x20080
				}
				0x3 => {
					/* rcr */

					0x20080
				}
				0x4 => {
					/* shl */

					0x20080
				}
				0x5 => {
					/* shr */

					0x20080
				}
				// 6 => capstone: rcr byte ptr [rdx], 1
				0x7 => {
					/* sar */

					0x20080
				}
				_ => 0,
			}
		}
		0xd1 => {
			match (try!(c.peek()) >> 3u8) & 7 { 
				0x0 => {
					/* rol */

					0x20090
				}
				0x1 => {
					/* ror */

					0x20090
				}
				0x2 => {
					/* rcl */

					0x20090
				}
				0x3 => {
					/* rcr */

					0x20090
				}
				0x4 => {
					/* shl */

					0x20090
				}
				0x5 => {
					/* shr */

					0x20090
				}
				// 6 => capstone: rcr dword ptr [rdx], 1
				0x7 => {
					/* sar */

					0x20090
				}
				_ => 0,
			}
		}
		0xd2 => {
			match (try!(c.peek()) >> 3u8) & 7 { 
				0x0 => {
					/* rol */

					0x20080
				}
				0x1 => {
					/* ror */

					0x20080
				}
				0x2 => {
					/* rcl */

					0x20080
				}
				0x3 => {
					/* rcr */

					0x20080
				}
				0x4 => {
					/* shl */

					0x20080
				}
				0x5 => {
					/* shr */

					0x20080
				}
				// 6 => capstone: rcr byte ptr [rdx], cl
				0x7 => {
					/* sar */

					0x20080
				}
				_ => 0,
			}
		}
		0xd3 => {
			match (try!(c.peek()) >> 3u8) & 7 { 
				0x0 => {
					/* rol */

					0x20090
				}
				0x1 => {
					/* ror */

					0x20090
				}
				0x2 => {
					/* rcl */

					0x20090
				}
				0x3 => {
					/* rcr */

					0x20090
				}
				0x4 => {
					/* shl */

					0x20090
				}
				0x5 => {
					/* shr */

					0x20090
				}
				// 6 => capstone: rcr dword ptr [rdx], cl
				0x7 => {
					/* sar */

					0x20090
				}
				_ => 0,
			}
		}
		0xe8 => {
			/* call */

			0x710
		}
		0xe9 => {
			/* jmp */

			0x790
		}
		0xeb => {
			/* jmp */

			0x810
		}
		0xf6 => {
			match (try!(c.peek()) >> 3u8) & 7 { 
				0x0 => {
					/* test */

					0x28180
				}
				// 1 => capstone: neg byte ptr [rdx]
				0x2 => {
					/* not */

					0x20081
				}
				0x3 => {
					/* neg */

					0x20081
				}
				0x4 => {
					/* mul */

					0x60180
				}
				0x5 => {
					/* imul */

					0x60180
				}
				0x6 => {
					/* div */

					0x60180
				}
				0x7 => {
					/* idiv */

					0x60180
				}
				_ => 0,
			}
		}
		0xf7 => {
			match (try!(c.peek()) >> 3u8) & 7 { 
				0x0 => {
					/* test */

					0x30198
				}
				// 1 => capstone: neg dword ptr [rdx]
				0x2 => {
					/* not */

					0x20091
				}
				0x3 => {
					/* neg */

					0x20091
				}
				0x4 => {
					/* mul */

					0xe0190
				}
				0x5 => {
					/* imul */

					0xe0190
				}
				0x6 => {
					/* div */

					0xe0190
				}
				0x7 => {
					/* idiv */

					0xe0190
				}
				_ => 0,
			}
		}
		0xfe => {
			match (try!(c.peek()) >> 3u8) & 7 { 
				0x0 => {
					/* inc */

					0x20081
				}
				0x1 => {
					/* dec */

					0x20081
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
			match (try!(c.peek()) >> 3u8) & 7 { 
				0x0 => {
					/* inc */

					0x20091
				}
				0x1 => {
					/* dec */

					0x20091
				}
				0x2 => {
					/* call */

					0x20090
				}
				// 3 => capstone: lcall ptr [rdx]
				0x4 => {
					/* jmp */

					0x20090
				}
				// 5 => capstone: lcall ptr [rdx]
				// 6 => capstone: lcall ptr [rdx]
				// 7 => capstone: lcall ptr [rdx]
				_ => 0,
			}
		}
		_ => 0,
	})
}
