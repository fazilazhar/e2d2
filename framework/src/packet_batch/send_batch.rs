use io::PmdPort;
use io::Result;
use super::act::Act;
use super::Batch;
use super::iterator::*;
use std::any::Any;

// FIXME: Should we be handling multiple queues and ports here?
// FIXME: Should this really even be a batch?
pub struct SendBatch<V>
    where V: Batch + BatchIterator + Act
{
    port: PmdPort,
    queue: i32,
    parent: V,
    pub sent: u64,
}

impl<V> SendBatch<V>
    where V: Batch + BatchIterator + Act
{
    pub fn new(parent: V, port: PmdPort, queue: i32) -> SendBatch<V> {
        SendBatch {
            port: port,
            queue: queue,
            sent: 0,
            parent: parent,
        }
    }

    pub fn process(&mut self) {
        self.act();
    }
}

impl<V> Batch for SendBatch<V> where V: Batch + BatchIterator + Act {}

impl<V> BatchIterator for SendBatch<V>
    where V: Batch + BatchIterator + Act
{
    #[inline]
    fn start(&mut self) -> usize {
        panic!("Cannot iterate SendBatch")
    }

    #[inline]
    unsafe fn next_payload(&mut self, _: usize) -> Option<(PacketDescriptor, Option<&mut Any>, usize)> {
        panic!("Cannot iterate SendBatch")
    }

    #[inline]
    unsafe fn next_base_payload(&mut self, _: usize) -> Option<(PacketDescriptor, Option<&mut Any>, usize)> {
        panic!("Cannot iterate SendBatch")
    }

    #[inline]
    unsafe fn next_payload_popped(&mut self, _: usize, _: i32) -> Option<(PacketDescriptor, Option<&mut Any>, usize)> {
        panic!("Cannot iterate SendBatch")
    }
}

/// Internal interface for packets.
impl<V> Act for SendBatch<V>
    where V: Batch + BatchIterator + Act
{
    #[inline]
    fn act(&mut self) {
        // First everything is applied
        self.parent.act();
        self.parent
            .send_queue(&mut self.port, self.queue)
            .and_then(|x| {
                self.sent += x as u64;
                Ok(x)
            })
            .expect("Send failed");
        self.parent.done();
    }

    fn done(&mut self) {}

    fn send_queue(&mut self, _: &mut PmdPort, _: i32) -> Result<u32> {
        panic!("Cannot send a sent packet batch")
    }

    fn capacity(&self) -> i32 {
        self.parent.capacity()
    }

    #[inline]
    fn drop_packets(&mut self, _: Vec<usize>) -> Option<usize> {
        panic!("Cannot drop packets from a sent batch")
    }

    #[inline]
    fn adjust_payload_size(&mut self, _: usize, _: isize) -> Option<isize> {
        panic!("Cannot resize a sent batch")
    }

    #[inline]
    fn adjust_headroom(&mut self, _: usize, _: isize) -> Option<isize> {
        panic!("Cannot resize a sent batch")
    }
}
