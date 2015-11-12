use memory::Page;
use memory::offset_mut;
use arch::PAGE_SIZE;
use arch::memory;
use util::LinkedList;

#[derive(Eq, PartialEq)]
pub enum Kind {
	Free,
	Overhead,
	Default,
	Stack,
	UserAllocator,
	PhysicalView,
}

pub struct Block {
	kind: Kind,
	pub base: usize,
	pub pages: usize,

	linear_prev: Option<*mut Block>,
	linear_next: Option<*mut Block>,

	// Free to be used when allocated

	list_prev: Option<*mut Block>,
	list_next: Option<*mut Block>,
}

pub struct Allocator {
    // List of blocks not representing any memory
	free_block_list: LinkedList<Block>,

    // List of blocks representing free memory
    free_list: LinkedList<Block>,

    // List of blocks representing free and used memory sorted by virtual address
	linear_list: LinkedList<Block>,

	current_block: *mut Block,
	end_block: *mut Block,

	first_block: Block,
}

impl Allocator {
    pub fn new(start: Page, end: Page) -> Allocator {
        use std::ptr::null_mut;

        let mut alloc = Allocator {
        	free_block_list: LinkedList::new(offset_of!(Block, list_prev), offset_of!(Block, list_next)),
            free_list: LinkedList::new(offset_of!(Block, list_prev), offset_of!(Block, list_next)),
        	linear_list: LinkedList::new(offset_of!(Block, linear_prev), offset_of!(Block, linear_next)),

        	current_block: null_mut(),
        	end_block: null_mut(),

        	first_block: Block {
                base: start.ptr() / PAGE_SIZE,
                pages: (end.ptr() - start.ptr()) / PAGE_SIZE,
                kind: Kind::Free,
            	linear_prev: None,
            	linear_next: None,
            	list_prev: None,
            	list_next: None,
            },
        };

        unsafe {
        	alloc.free_list.append(&mut alloc.first_block as *mut Block);
        	alloc.linear_list.append(&mut alloc.first_block as *mut Block);
        }

        alloc
    }

    unsafe fn allocate_block(&mut self) -> *mut Block {
        // Check if any free block is available

        if let Some(block) = self.free_block_list.first {
            self.free_block_list.remove(block);
            return block;
        }

        // Do we have an available block in our block array?

        if offset_mut(self.current_block, 1) < self.end_block {
            let result = self.current_block;
            self.current_block = offset_mut(self.current_block, 1);
            return result;
        }

        // Steal a page from the first free block and use it for a new block array

        let free = &mut *self.free_list.first.unwrap_or_else(|| panic!("Out of virtual memory"));

        assert!(free.pages != 0, "Empty block found");

        free.pages -= 1;

        let overhead = (free.base * PAGE_SIZE) as usize;
        free.base += 1;

        // Mark this page as used

        let overhead_block = &mut *(overhead as *mut Block);

        self.current_block = offset_mut(overhead_block, 1);
        self.end_block = (free.base * PAGE_SIZE) as usize as *mut Block;

        assert!(self.current_block < self.end_block, "Overflow");

        memory::map(Page::new(overhead), 1, memory::RW_DATA_FLAGS);

        overhead_block.kind = Kind::Overhead;
        overhead_block.base = overhead;
        overhead_block.pages = 1;

        self.linear_list.insert_before(overhead_block, free);

        if free.pages == 0 {
            // The block we stole a page from is empty so we can reuse it
            self.linear_list.remove(free);
            self.free_list.remove(free);
            return free;
        }

        let result = self.current_block;
        self.current_block = offset_mut(self.current_block, 1);

        return result;
    }

    pub fn allocate(&mut self, kind: Kind, pages: usize) -> *mut Block {
    	assert!(pages > 0, "Can't allocate zero pages");

        unsafe {
        	let result = &mut *self.allocate_block(); // Allocate a result block first since it can modify free regions

            println!("ALLOC {}", pages);

            let mut c = self.free_list.first;
            loop {
                let current = &mut *match c {
                    Some(v) => v,
                    None => break
                };

                println!("FREE LIST block @ {:#x} - {:#x} - {:#x}", current as *mut Block as usize, current.base * PAGE_SIZE, (current.base + current.pages) * PAGE_SIZE);

        		if current.pages >= pages { // We have a winner
        			if current.pages == pages { // It fits perfectly
        				self.free_block_list.append(result);
        				self.free_list.remove(current);

        				current.kind = kind;

        				return current;
        			}

        			self.linear_list.insert_before(result, current);

        			result.kind = kind;
        			result.base = current.base;
        			result.pages = pages;
        			current.base += pages;
        			current.pages -= pages;

                    println!("allocate block @ {:#x} - {:#x} - {:#x}", result as *mut Block as usize, (*result).base * PAGE_SIZE, ((*result).base + pages) * PAGE_SIZE);

        			return result;
        		}

        		c = current.list_next;
        	}

        	panic!("Out of virtual memory");
        }
    }

    pub fn free(&mut self, block: *mut Block) {
        unsafe {
            println!("free block @ {:#x} - {:#x} - {:#x}", block as usize, (*block).base * PAGE_SIZE, ((*block).base + (*block).pages) * PAGE_SIZE);

            let block = &mut *block;

        	if block.kind == Kind::PhysicalView {
        		memory::unmap_view(Page::new(block.base * PAGE_SIZE), block.pages);
        	} else {
        		memory::unmap(Page::new(block.base * PAGE_SIZE), block.pages);
            }

        	let prev = block.linear_prev;
        	let next = block.linear_next;
        	let mut current = &mut *(block as *mut Block);

        	// Merge with a block below

        	if let Some(prev) = prev {
                let prev = &mut *prev;
                if prev.kind == Kind::Free {
            		assert!(prev.base + prev.pages == current.base);
            		prev.pages += current.pages;

            		self.free_block_list.append(current);
            		self.linear_list.remove(current);

            		current = prev;
                }
        	}

        	// Merge with a block above

        	if let Some(next) = next {
                let next = &mut *next;
                if next.kind == Kind::Free {
            		assert!(current.base + current.pages == next.base);

            		next.base -= current.pages;
            		next.pages += current.pages;

            		if Some(current as *mut Block) == prev {
            			self.free_list.remove(current);
                    }

            		self.free_block_list.append(current);
            		self.linear_list.remove(current);

            		current = next;
                }
        	}

        	if current as *mut Block == block {
        		current.kind = Kind::Free;
        		self.free_list.append(current);
        	}
        }
    }
}
