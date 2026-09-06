#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    FrontLevel,
    FrontCct,
    RearLevel,
    RearLook,
    RearSpeed,
}

const SLOTS: [Slot; 5] = [
    Slot::FrontLevel,
    Slot::FrontCct,
    Slot::RearLevel,
    Slot::RearLook,
    Slot::RearSpeed,
];

#[derive(Default)]
pub struct Pending {
    marked: [bool; 5],
}

impl Pending {
    pub fn mark(&mut self, slot: Slot) {
        self.marked[index(slot)] = true;
    }

    pub fn take_slot(&mut self) -> Option<Slot> {
        for slot in SLOTS {
            let i = index(slot);
            if self.marked[i] {
                self.marked[i] = false;
                return Some(slot);
            }
        }
        None
    }

    pub fn is_empty(&self) -> bool {
        self.marked.iter().all(|m| !m)
    }
}

const fn index(slot: Slot) -> usize {
    match slot {
        Slot::FrontLevel => 0,
        Slot::FrontCct => 1,
        Slot::RearLevel => 2,
        Slot::RearLook => 3,
        Slot::RearSpeed => 4,
    }
}
