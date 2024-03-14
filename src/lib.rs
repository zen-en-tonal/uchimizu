#[cfg(not(feature = "serde"))]
type Instant = std::time::Instant;
#[cfg(feature = "serde")]
type Instant = chrono::DateTime<chrono::Utc>;

#[cfg(not(feature = "serde"))]
type Duration = std::time::Duration;
#[cfg(feature = "serde")]
type Duration = chrono::TimeDelta;

fn now() -> Instant {
    #[cfg(not(feature = "serde"))]
    return std::time::Instant::now();
    #[cfg(feature = "serde")]
    return chrono::Utc::now();
}

fn duration_secs(d: Duration) -> u32 {
    #[cfg(not(feature = "serde"))]
    return d.as_secs().try_into().unwrap();
    #[cfg(feature = "serde")]
    return d.num_seconds().try_into().unwrap();
}

#[cfg_attr(not(feature = "serde"), derive(Debug, Clone, PartialEq, Eq))]
#[cfg_attr(
    feature = "serde",
    derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)
)]
pub struct Policy {
    initial_amount: u32,
    pour_cost: u32,
    evaporation_cost: u32,
}

impl Policy {
    pub fn new(initial_amount: u32, pour_cost: u32, evaporation_cost: u32) -> Policy {
        Policy {
            initial_amount,
            pour_cost,
            evaporation_cost,
        }
    }

    /// # Example
    /// ```
    /// use uchimizu::Policy;
    ///
    /// let p = Policy::bottom_less();
    /// assert!(p.is_remaining(0, 0));
    /// assert!(p.is_remaining(0, 1));
    /// assert!(p.is_remaining(1, 1));
    /// ```
    pub fn bottom_less() -> Policy {
        Policy {
            initial_amount: 1,
            pour_cost: 0,
            evaporation_cost: 0,
        }
    }

    /// # Example
    /// ```
    /// use uchimizu::Policy;
    ///
    /// let p = Policy::pierced();
    /// assert!(!p.is_remaining(0, 0));
    /// assert!(!p.is_remaining(0, 1));
    /// assert!(!p.is_remaining(1, 1));
    /// ```
    pub fn pierced() -> Policy {
        Policy {
            initial_amount: 0,
            pour_cost: 1,
            evaporation_cost: 1,
        }
    }

    /// # Example
    /// ```
    /// use uchimizu::Policy;
    ///
    /// let p = Policy::expire_within_counts(5);
    /// assert!(p.is_remaining(4, 1000));
    /// assert!(!p.is_remaining(5, 1000));
    /// assert!(!p.is_remaining(6, 1000));
    /// ```
    pub fn expire_within_counts(count: u32) -> Policy {
        Policy {
            initial_amount: count,
            pour_cost: 1,
            evaporation_cost: 0,
        }
    }

    /// # Example
    /// ```
    /// use uchimizu::Policy;
    ///
    /// let p = Policy::expire_within_secs(5);
    /// assert!(p.is_remaining(1000, 4));
    /// assert!(!p.is_remaining(1000, 5));
    /// assert!(!p.is_remaining(1000, 6));
    /// ```
    pub fn expire_within_secs(secs: u32) -> Policy {
        Policy {
            initial_amount: secs,
            pour_cost: 0,
            evaporation_cost: 1,
        }
    }

    pub fn is_remaining(&self, hit_count: usize, duration_secs: u32) -> bool {
        let pour_amount = self.pour_cost * hit_count as u32;
        let evaporation_amount = self.evaporation_cost * duration_secs;
        pour_amount + evaporation_amount < self.initial_amount
    }

    pub fn into_bucket<T>(self) -> Bucket<T> {
        Bucket {
            cache: None,
            policy: self,
            hit_count: 0,
            initiate: now(),
        }
    }
}

#[cfg_attr(not(feature = "serde"), derive(Debug, Clone, PartialEq, Eq))]
#[cfg_attr(
    feature = "serde",
    derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)
)]
pub struct Bucket<T> {
    cache: Option<T>,
    policy: Policy,
    hit_count: usize,
    initiate: Instant,
}

pub trait Task<T> {
    fn call(&self) -> T;
}

impl<F, T> Task<T> for F
where
    F: Fn() -> T,
{
    fn call(&self) -> T {
        (self)()
    }
}

impl<T> AsRef<Policy> for Bucket<T> {
    fn as_ref(&self) -> &Policy {
        &self.policy
    }
}

impl<T> Bucket<T>
where
    T: Clone,
{
    pub fn call<F>(&mut self, task: F) -> T
    where
        F: Task<T>,
    {
        match (
            self.policy
                .is_remaining(self.hit_count, duration_secs(now() - self.initiate)),
            self.cache.clone(),
        ) {
            (true, Some(c)) => {
                self.hit_count += 1;
                c
            }
            (_, _) => {
                self.refresh();
                self.hit_count += 1;
                let entry = task.call();
                self.cache = Some(entry.clone());
                entry
            }
        }
    }

    pub fn refresh(&mut self) {
        self.hit_count = 0;
        self.cache = None;
        self.initiate = now();
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;

    #[test]
    fn it_works() {
        let mut b = Policy::new(100, 10, 50).into_bucket();
        b.call(|| now());
    }

    #[test]
    fn bottom_less_works() {
        let mut b = Policy::bottom_less().into_bucket();
        b.call(|| now());
    }

    #[test]
    fn pierced_works() {
        let mut b = Policy::pierced().into_bucket();
        b.call(|| now());
    }

    #[test]
    fn expire_within_secs_works() {
        let mut b = Policy::expire_within_secs(1).into_bucket();
        b.call(|| now());
    }

    #[test]
    fn expire_within_counts_works() {
        let mut b = Policy::expire_within_counts(1).into_bucket();
        b.call(|| now());
    }
}
