use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
};

use super::AttachmentId;
use crate::bridge::{EVENT_CAPACITY, Error};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Owner {
    Script(AttachmentId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Binding {
    player: u64,
    connection: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClaimResource {
    ConsumableSlot5 { item: Option<i64> },
}

struct HeldClaim {
    owner: Owner,
    binding: Binding,
    resource: ClaimResource,
}

pub struct ClaimGrant {
    pub id: u64,
    pub first: bool,
}

pub(crate) struct Claims {
    binding: Binding,
    holders: HashMap<u64, HeldClaim>,
    next: u64,
}

impl Default for Claims {
    fn default() -> Self {
        static NEXT_PLAYER: AtomicU64 = AtomicU64::new(1);
        Self {
            binding: Binding {
                player: NEXT_PLAYER.fetch_add(1, Ordering::Relaxed),
                connection: 0,
            },
            holders: HashMap::new(),
            next: 0,
        }
    }
}

impl Claims {
    pub(crate) fn binding(&self) -> Binding {
        self.binding
    }

    pub(crate) fn acquire(
        &mut self,
        owner: Owner,
        resource: ClaimResource,
    ) -> Result<Option<ClaimGrant>, Error> {
        if self
            .holders
            .values()
            .next()
            .is_some_and(|held| held.resource != resource)
        {
            return Ok(None);
        }
        if self.holders.len() >= EVENT_CAPACITY {
            return Err(Error::Overflow);
        }
        let first = self.holders.is_empty();
        self.next = self.next.checked_add(1).ok_or(Error::Overflow)?;
        self.holders.insert(
            self.next,
            HeldClaim {
                owner,
                binding: self.binding,
                resource,
            },
        );
        Ok(Some(ClaimGrant {
            id: self.next,
            first,
        }))
    }

    pub(crate) fn release(&mut self, owner: Owner, id: u64) {
        if self
            .holders
            .get(&id)
            .is_some_and(|held| held.owner == owner && held.binding == self.binding)
        {
            self.holders.remove(&id);
        }
    }

    pub(crate) fn release_owner(&mut self, owner: Owner) {
        self.holders.retain(|_, held| held.owner != owner);
    }

    pub(crate) fn clear(&mut self) {
        self.holders.clear();
        self.binding.connection += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compatible_holders_share_until_the_last_release_and_stale_releases_do_nothing() {
        let mut claims = Claims::default();
        let a = Owner::Script(super::super::AttachmentId(1));
        let b = Owner::Script(super::super::AttachmentId(2));
        let resource = ClaimResource::ConsumableSlot5 { item: Some(100) };
        let first = claims.acquire(a, resource).unwrap().unwrap();
        let shared = claims.acquire(b, resource).unwrap().unwrap();
        assert!(first.first);
        assert!(!shared.first);
        assert!(
            claims
                .acquire(b, ClaimResource::ConsumableSlot5 { item: Some(200) })
                .unwrap()
                .is_none()
        );
        claims.release(b, first.id);
        claims.release(a, first.id);
        assert!(
            claims
                .acquire(a, ClaimResource::ConsumableSlot5 { item: Some(200) })
                .unwrap()
                .is_none()
        );
        claims.release_owner(b);
        let next = claims
            .acquire(b, ClaimResource::ConsumableSlot5 { item: Some(200) })
            .unwrap()
            .unwrap();
        assert!(next.first);
        claims.release(a, first.id);
        claims.release(b, shared.id);
        assert!(claims.acquire(a, resource).unwrap().is_none());
    }

    #[test]
    fn only_the_owner_can_release_a_holder() {
        let mut claims = Claims::default();
        let owner = Owner::Script(super::super::AttachmentId(1));
        let other = Owner::Script(super::super::AttachmentId(2));
        let first = claims
            .acquire(owner, ClaimResource::ConsumableSlot5 { item: Some(100) })
            .unwrap()
            .unwrap();
        claims.release(other, first.id);
        assert!(
            claims
                .acquire(other, ClaimResource::ConsumableSlot5 { item: Some(200) })
                .unwrap()
                .is_none()
        );
        claims.release(owner, first.id);
        assert!(
            claims
                .acquire(other, ClaimResource::ConsumableSlot5 { item: Some(200) })
                .unwrap()
                .unwrap()
                .first
        );
    }

    #[test]
    fn ambient_holders_share_and_invalidation_frees_the_slot() {
        let mut claims = Claims::default();
        let a = Owner::Script(super::super::AttachmentId(1));
        let resource = ClaimResource::ConsumableSlot5 { item: None };
        assert!(claims.acquire(a, resource).unwrap().unwrap().first);
        assert!(!claims.acquire(a, resource).unwrap().unwrap().first);
        assert!(
            claims
                .acquire(a, ClaimResource::ConsumableSlot5 { item: Some(100) })
                .unwrap()
                .is_none()
        );
        claims.clear();
        assert!(
            claims
                .acquire(a, ClaimResource::ConsumableSlot5 { item: Some(100) })
                .unwrap()
                .unwrap()
                .first
        );
    }
}
