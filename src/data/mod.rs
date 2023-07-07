use std::collections::HashMap;
use std::hash::Hash;
use std::ops::AddAssign;

pub(crate) mod compileroutput;
pub(crate) mod llvmcovdata;
pub(crate) mod project;
pub(crate) mod syn_visit;

trait Update<K, V>
    where K: Eq + Hash {
    fn add_or_insert(&mut self, key: K, value: V);
}

impl<K, V> Update<K, V> for HashMap<K, V>
    where K: Eq + Hash, V: AddAssign + Default {
    fn add_or_insert(&mut self, key: K, value: V) {
        *self.entry(key).or_insert(V::default()) += value;
    }
}
