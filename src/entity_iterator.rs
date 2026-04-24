use cranelift_entity::{EntityList, EntityRef, ListPool, packed_option::ReservedValue};

pub struct EntityIter<T: EntityRef + ReservedValue> {
    i: usize,
    entities: EntityList<T>,
}

impl<T: EntityRef + ReservedValue> EntityIter<T> {
    pub fn from(entities: EntityList<T>) -> Self {
        Self { i: 0, entities }
    }

    pub fn next(&mut self, pool: &ListPool<T>) -> Option<T> {
        let v = self.entities.get(self.i, pool);
        self.i += 1;
        v
    }
}
