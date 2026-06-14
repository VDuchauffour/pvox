pub struct SparkLineData {
    pub cpu_history: Vec<u64>,
    pub mem_history: Vec<u64>,
}

impl SparkLineData {
    pub fn new() -> Self {
        Self {
            cpu_history: Vec::with_capacity(60),
            mem_history: Vec::with_capacity(60),
        }
    }

    pub fn push_cpu(&mut self, value: u64) {
        if self.cpu_history.len() >= 60 {
            self.cpu_history.remove(0);
        }
        self.cpu_history.push(value);
    }

    pub fn push_mem(&mut self, value: u64) {
        if self.mem_history.len() >= 60 {
            self.mem_history.remove(0);
        }
        self.mem_history.push(value);
    }

    pub fn clear(&mut self) {
        self.cpu_history.clear();
        self.mem_history.clear();
    }
}

impl Default for SparkLineData {
    fn default() -> Self {
        Self::new()
    }
}
