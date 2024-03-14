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

fn duration_secs(d: Duration) -> i64 {
    #[cfg(not(feature = "serde"))]
    return d.as_secs() as i64;
    #[cfg(feature = "serde")]
    return d.num_seconds();
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

pub fn new_policy(initial_amount: u32, pour_cost: u32, evaporation_cost: u32) -> Policy {
    Policy {
        initial_amount,
        pour_cost,
        evaporation_cost,
    }
}

impl Policy {
    fn is_remaining(&self, hit_count: usize, duration: Duration) -> bool {
        let pour_amount = self.pour_cost * hit_count as u32;
        let evaporation_amount = self.evaporation_cost * duration_secs(duration) as u32;
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
        let duration = now() - self.initiate;
        match (
            self.policy.is_remaining(self.hit_count, duration),
            self.cache.clone(),
        ) {
            (true, Some(c)) => {
                self.hit_count += 1;
                c
            }
            (false, _) => {
                self.refresh();
                self.call(task)
            }
            (_, None) => {
                self.cache = Some(task.call());
                self.call(task)
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
        let mut b = new_policy(100, 10, 50).into_bucket();
        for _ in 0..2 {
            println!(
                "{:?}",
                (
                    b.call(|| {
                        println!("called");
                        now()
                    }),
                    b.hit_count,
                    b.initiate
                )
            );
            thread::sleep(std::time::Duration::from_millis(500));
        }
    }
}
