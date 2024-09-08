
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum WhichBuffer {
    A,
    B,
    C,
}

impl WhichBuffer {
    fn next(self) -> Self {
        match self {
            WhichBuffer::A => WhichBuffer::B,
            WhichBuffer::B => WhichBuffer::C,
            WhichBuffer::C => WhichBuffer::A,
        }
    }
}


#[derive(Debug)]
pub struct CommChannel {
    // The cc values since last time.  Note we use a tripple-buffer system so that we can
    // always read freely from a buffer while writing freely to another. 
    cc_values_a: VecDeque<CcValueTime>,
    cc_values_b: VecDeque<CcValueTime>,
    cc_values_c: VecDeque<CcValueTime>,
    write_queue: AtomicCell<WhichBuffer>,
    read_queue:  AtomicCell<WhichBuffer>,
}

impl Default for CommChannel {
    fn default() -> Self {
        Self { 
            cc_values_a: Default::default(), 
            cc_values_b: Default::default(), 
            cc_values_c: Default::default(), 
            write_queue: AtomicCell::new(WhichBuffer::B), 
            read_queue:  AtomicCell::new(WhichBuffer::A),
        }
    }
}

impl CommChannel {
    fn get_buffer_mut(&mut self, which: WhichBuffer) -> &mut VecDeque<CcValueTime> {
        match which {
            WhichBuffer::A => &mut self.cc_values_a,
            WhichBuffer::B => &mut self.cc_values_b,
            WhichBuffer::C => &mut self.cc_values_c,
        }
    }

    pub fn push(&mut self, val: CcValueTime) {
        self.get_buffer_mut(self.write_queue.load()).push_back(val);
    }

    pub fn pushes_completed(&mut self) {
        let read_queue  = self.read_queue.load();
        let write_queue = self.write_queue.load();
        if write_queue.next() != read_queue {
            self.write_queue.store(write_queue.next());   // Advance to the next queue.
        }
    }

    pub fn pop(&mut self) -> Option<CcValueTime> {
        let write_queue = self.write_queue.load();
        let read_queue  = self.read_queue.load();
        let val = self.get_buffer_mut(read_queue).pop_front();

        // If this queue is empty, advance to the next queue, if available, and pop() from there
        if val.is_none() && read_queue.next() != write_queue {
            self.read_queue.store(read_queue.next());   // Advance to the next queue...
            return self.pop();                          // ...and return value from there
        }
        val
    }
}