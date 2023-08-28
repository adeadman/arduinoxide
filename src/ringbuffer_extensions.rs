use ringbuffer::{RingBuffer, ConstGenericRingBuffer};


trait CollectionAverage<S> {
    fn average(&self) -> S;
}

impl<S, T, const CAP: usize> CollectionAverage<S> for ConstGenericRingBuffer::<T, CAP> 
where
    T: Eq + Copy,
    S: for<'a> core::iter::Sum<&'a T> + core::ops::Div<Output = S> + core::convert::From<u16>,
{
    fn average(&self) -> S {
        let divisor = S::try_from(self.len() as u16).unwrap();
        self.iter().sum::<S>() / divisor
    }
}
