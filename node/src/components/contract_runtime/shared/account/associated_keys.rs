use std::collections::{BTreeMap, BTreeSet};

use casperlabs_types::{
    account::{
        AccountHash, AddKeyFailure, RemoveKeyFailure, UpdateKeyFailure, Weight, MAX_ASSOCIATED_KEYS,
    },
    bytesrepr::{Error, FromBytes, ToBytes},
};

#[derive(Default, PartialOrd, Ord, PartialEq, Eq, Clone, Debug)]
pub struct AssociatedKeys(BTreeMap<AccountHash, Weight>);

impl AssociatedKeys {
    pub fn new(key: AccountHash, weight: Weight) -> AssociatedKeys {
        let mut bt: BTreeMap<AccountHash, Weight> = BTreeMap::new();
        bt.insert(key, weight);
        AssociatedKeys(bt)
    }

    /// Adds new AssociatedKey to the set.
    /// Returns true if added successfully, false otherwise.
    #[allow(clippy::map_entry)]
    pub fn add_key(&mut self, key: AccountHash, weight: Weight) -> Result<(), AddKeyFailure> {
        if self.0.len() == MAX_ASSOCIATED_KEYS {
            Err(AddKeyFailure::MaxKeysLimit)
        } else if self.0.contains_key(&key) {
            Err(AddKeyFailure::DuplicateKey)
        } else {
            self.0.insert(key, weight);
            Ok(())
        }
    }

    /// Removes key from the associated keys set.
    /// Returns true if value was found in the set prior to the removal, false
    /// otherwise.
    pub fn remove_key(&mut self, key: &AccountHash) -> Result<(), RemoveKeyFailure> {
        self.0
            .remove(key)
            .map(|_| ())
            .ok_or(RemoveKeyFailure::MissingKey)
    }

    /// Adds new AssociatedKey to the set.
    /// Returns true if added successfully, false otherwise.
    #[allow(clippy::map_entry)]
    pub fn update_key(&mut self, key: AccountHash, weight: Weight) -> Result<(), UpdateKeyFailure> {
        if !self.0.contains_key(&key) {
            return Err(UpdateKeyFailure::MissingKey);
        }

        self.0.insert(key, weight);
        Ok(())
    }

    pub fn get(&self, key: &AccountHash) -> Option<&Weight> {
        self.0.get(key)
    }

    pub fn contains_key(&self, key: &AccountHash) -> bool {
        self.0.contains_key(key)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&AccountHash, &Weight)> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Helper method that calculates weight for keys that comes from any
    /// source.
    ///
    /// This method is not concerned about uniqueness of the passed iterable.
    /// Uniqueness is determined based on the input collection properties,
    /// which is either BTreeSet (in [`AssociatedKeys::calculate_keys_weight`])
    /// or BTreeMap (in [`AssociatedKeys::total_keys_weight`]).
    fn calculate_any_keys_weight<'a>(&self, keys: impl Iterator<Item = &'a AccountHash>) -> Weight {
        let total = keys
            .filter_map(|key| self.0.get(key))
            .fold(0u8, |acc, w| acc.saturating_add(w.value()));

        Weight::new(total)
    }

    /// Calculates total weight of authorization keys provided by an argument
    pub fn calculate_keys_weight(&self, authorization_keys: &BTreeSet<AccountHash>) -> Weight {
        self.calculate_any_keys_weight(authorization_keys.iter())
    }

    /// Calculates total weight of all authorization keys
    pub fn total_keys_weight(&self) -> Weight {
        self.calculate_any_keys_weight(self.0.keys())
    }

    /// Calculates total weight of all authorization keys excluding a given key
    pub fn total_keys_weight_excluding(&self, account_hash: AccountHash) -> Weight {
        self.calculate_any_keys_weight(self.0.keys().filter(|&&element| element != account_hash))
    }
}

impl ToBytes for AssociatedKeys {
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        self.0.to_bytes()
    }

    fn serialized_length(&self) -> usize {
        self.0.serialized_length()
    }
}

impl FromBytes for AssociatedKeys {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), Error> {
        let (keys_map, rem) = BTreeMap::<AccountHash, Weight>::from_bytes(bytes)?;
        let mut keys = AssociatedKeys::default();
        keys_map.into_iter().for_each(|(k, v)| {
            // NOTE: we're ignoring potential errors (duplicate key, maximum number of
            // elements). This is safe, for now, as we were the ones that
            // serialized `AssociatedKeys` in the first place.
            keys.add_key(k, v).unwrap();
        });
        Ok((keys, rem))
    }
}

#[cfg(any(feature = "gens", test))]
pub mod gens {
    use proptest::prelude::*;

    use casperlabs_types::gens::{account_hash_arb, weight_arb};

    use super::AssociatedKeys;

    pub fn associated_keys_arb(size: usize) -> impl Strategy<Value = AssociatedKeys> {
        proptest::collection::btree_map(account_hash_arb(), weight_arb(), size).prop_map(|keys| {
            let mut associated_keys = AssociatedKeys::default();
            keys.into_iter().for_each(|(k, v)| {
                associated_keys.add_key(k, v).unwrap();
            });
            associated_keys
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeSet, iter::FromIterator};

    use casperlabs_types::{
        account::{AccountHash, AddKeyFailure, Weight, ACCOUNT_HASH_LENGTH, MAX_ASSOCIATED_KEYS},
        bytesrepr,
    };

    use super::AssociatedKeys;

    #[test]
    fn associated_keys_add() {
        let mut keys =
            AssociatedKeys::new(AccountHash::new([0u8; ACCOUNT_HASH_LENGTH]), Weight::new(1));
        let new_pk = AccountHash::new([1u8; ACCOUNT_HASH_LENGTH]);
        let new_pk_weight = Weight::new(2);
        assert!(keys.add_key(new_pk, new_pk_weight).is_ok());
        assert_eq!(keys.get(&new_pk), Some(&new_pk_weight))
    }

    #[test]
    fn associated_keys_add_full() {
        let map = (0..MAX_ASSOCIATED_KEYS).map(|k| {
            (
                AccountHash::new([k as u8; ACCOUNT_HASH_LENGTH]),
                Weight::new(k as u8),
            )
        });
        assert_eq!(map.len(), 10);
        let mut keys = {
            let mut tmp = AssociatedKeys::default();
            map.for_each(|(key, weight)| assert!(tmp.add_key(key, weight).is_ok()));
            tmp
        };
        assert_eq!(
            keys.add_key(
                AccountHash::new([100u8; ACCOUNT_HASH_LENGTH]),
                Weight::new(100)
            ),
            Err(AddKeyFailure::MaxKeysLimit)
        )
    }

    #[test]
    fn associated_keys_add_duplicate() {
        let pk = AccountHash::new([0u8; ACCOUNT_HASH_LENGTH]);
        let weight = Weight::new(1);
        let mut keys = AssociatedKeys::new(pk, weight);
        assert_eq!(
            keys.add_key(pk, Weight::new(10)),
            Err(AddKeyFailure::DuplicateKey)
        );
        assert_eq!(keys.get(&pk), Some(&weight));
    }

    #[test]
    fn associated_keys_remove() {
        let pk = AccountHash::new([0u8; ACCOUNT_HASH_LENGTH]);
        let weight = Weight::new(1);
        let mut keys = AssociatedKeys::new(pk, weight);
        assert!(keys.remove_key(&pk).is_ok());
        assert!(keys
            .remove_key(&AccountHash::new([1u8; ACCOUNT_HASH_LENGTH]))
            .is_err());
    }

    #[test]
    fn associated_keys_calculate_keys_once() {
        let key_1 = AccountHash::new([0; 32]);
        let key_2 = AccountHash::new([1; 32]);
        let key_3 = AccountHash::new([2; 32]);
        let mut keys = AssociatedKeys::default();

        keys.add_key(key_2, Weight::new(2))
            .expect("should add key_1");
        keys.add_key(key_1, Weight::new(1))
            .expect("should add key_1");
        keys.add_key(key_3, Weight::new(3))
            .expect("should add key_1");

        assert_eq!(
            keys.calculate_keys_weight(&BTreeSet::from_iter(vec![
                key_1, key_2, key_3, key_1, key_2, key_3,
            ])),
            Weight::new(1 + 2 + 3)
        );
    }

    #[test]
    fn associated_keys_total_weight() {
        let associated_keys = {
            let mut res = AssociatedKeys::new(AccountHash::new([1u8; 32]), Weight::new(1));
            res.add_key(AccountHash::new([2u8; 32]), Weight::new(11))
                .expect("should add key 1");
            res.add_key(AccountHash::new([3u8; 32]), Weight::new(12))
                .expect("should add key 2");
            res.add_key(AccountHash::new([4u8; 32]), Weight::new(13))
                .expect("should add key 3");
            res
        };
        assert_eq!(
            associated_keys.total_keys_weight(),
            Weight::new(1 + 11 + 12 + 13)
        );
    }

    #[test]
    fn associated_keys_total_weight_excluding() {
        let identity_key = AccountHash::new([1u8; 32]);
        let identity_key_weight = Weight::new(1);

        let key_1 = AccountHash::new([2u8; 32]);
        let key_1_weight = Weight::new(11);

        let key_2 = AccountHash::new([3u8; 32]);
        let key_2_weight = Weight::new(12);

        let key_3 = AccountHash::new([4u8; 32]);
        let key_3_weight = Weight::new(13);

        let associated_keys = {
            let mut res = AssociatedKeys::new(identity_key, identity_key_weight);
            res.add_key(key_1, key_1_weight).expect("should add key 1");
            res.add_key(key_2, key_2_weight).expect("should add key 2");
            res.add_key(key_3, key_3_weight).expect("should add key 3");
            res
        };
        assert_eq!(
            associated_keys.total_keys_weight_excluding(key_2),
            Weight::new(identity_key_weight.value() + key_1_weight.value() + key_3_weight.value())
        );
    }

    #[test]
    fn overflowing_keys_weight() {
        let identity_key = AccountHash::new([1u8; 32]);
        let key_1 = AccountHash::new([2u8; 32]);
        let key_2 = AccountHash::new([3u8; 32]);
        let key_3 = AccountHash::new([4u8; 32]);

        let identity_key_weight = Weight::new(250);
        let weight_1 = Weight::new(1);
        let weight_2 = Weight::new(2);
        let weight_3 = Weight::new(3);

        let saturated_weight = Weight::new(u8::max_value());

        let associated_keys = {
            let mut res = AssociatedKeys::new(identity_key, identity_key_weight);

            res.add_key(key_1, weight_1).expect("should add key 1");
            res.add_key(key_2, weight_2).expect("should add key 2");
            res.add_key(key_3, weight_3).expect("should add key 3");
            res
        };

        assert_eq!(
            associated_keys.calculate_keys_weight(&BTreeSet::from_iter(vec![
                identity_key, // 250
                key_1,        // 251
                key_2,        // 253
                key_3,        // 256 - error
            ])),
            saturated_weight,
        );
    }

    #[test]
    fn serialization_roundtrip() {
        let mut keys = AssociatedKeys::default();
        keys.add_key(AccountHash::new([1; 32]), Weight::new(1))
            .unwrap();
        keys.add_key(AccountHash::new([2; 32]), Weight::new(2))
            .unwrap();
        keys.add_key(AccountHash::new([3; 32]), Weight::new(3))
            .unwrap();
        bytesrepr::test_serialization_roundtrip(&keys);
    }
}
