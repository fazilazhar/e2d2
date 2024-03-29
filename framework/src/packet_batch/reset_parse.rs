use io::PmdPort;
use io::Result;
use super::act::Act;
use super::Batch;
use super::iterator::*;
use std::any::Any;

// FIXME: Reconsider this choice some day
/// This is really the same thing as composition except that by accepting a template it is somewhat faster (since we
/// don't need dynamic dispatch).
pub struct ResetParsingBatch<V>
    where V: Batch + BatchIterator + Act
{
    parent: V,
}

impl<V> ResetParsingBatch<V>
    where V: Batch + BatchIterator + Act
{
    pub fn new(parent: V) -> ResetParsingBatch<V> {
        ResetParsingBatch { parent: parent }

    }
}

impl<V> Batch for ResetParsingBatch<V> where V: Batch + BatchIterator + Act {}

impl<V> BatchIterator for ResetParsingBatch<V>
    where V: Batch + BatchIterator + Act
{
    #[inline]
    fn start(&mut self) -> usize {
        self.parent.start()
    }

    #[inline]
    unsafe fn next_payload(&mut self, idx: usize) -> Option<(PacketDescriptor, Option<&mut Any>, usize)> {
        self.parent.next_base_payload(idx)
    }

    #[inline]
    unsafe fn next_base_payload(&mut self, idx: usize) -> Option<(PacketDescriptor, Option<&mut Any>, usize)> {
        self.parent.next_base_payload(idx)
    }

    #[inline]
    unsafe fn next_payload_popped(&mut self, _: usize, _: i32) -> Option<(PacketDescriptor, Option<&mut Any>, usize)> {
        panic!("Cannot pop past a rest operation")
    }
}

/// Internal interface for packets.
impl<V> Act for ResetParsingBatch<V>
    where V: Batch + BatchIterator + Act
{
    #[inline]
    fn act(&mut self) {
        self.parent.act();
    }

    #[inline]
    fn done(&mut self) {
        self.parent.done();
    }

    #[inline]
    fn send_queue(&mut self, port: &mut PmdPort, queue: i32) -> Result<u32> {
        self.parent.send_queue(port, queue)
    }

    #[inline]
    fn capacity(&self) -> i32 {
        self.parent.capacity()
    }

    #[inline]
    fn drop_packets(&mut self, idxes: Vec<usize>) -> Option<usize> {
        self.parent.drop_packets(idxes)
    }

    #[inline]
    fn adjust_payload_size(&mut self, idx: usize, size: isize) -> Option<isize> {
        self.parent.adjust_payload_size(idx, size)
    }

    #[inline]
    fn adjust_headroom(&mut self, idx: usize, size: isize) -> Option<isize> {
        self.parent.adjust_headroom(idx, size)
    }
}
