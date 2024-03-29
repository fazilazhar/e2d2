use super::mbuf::MBuf;
use super::interface::Result;
use super::interface::ZCSIError;
use super::super::headers::MacAddress;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// External DPDK calls
#[link(name = "zcsi")]
extern "C" {
    fn init_pmd_port(port: i32,
                     rxqs: i32,
                     txqs: i32,
                     rx_cores: *const i32,
                     tx_cores: *const i32,
                     nrxd: i32,
                     ntxd: i32,
                     loopback: i32,
                     tso: i32,
                     csumoffload: i32)
                     -> i32;
    fn free_pmd_port(port: i32) -> i32;
    fn recv_pkts(port: i32, qid: i32, pkts: *mut *mut MBuf, len: i32) -> i32;
    fn send_pkts(port: i32, qid: i32, pkts: *mut *mut MBuf, len: i32) -> i32;
    fn num_pmd_ports() -> i32;
    fn rte_eth_macaddr_get(port: i32, address: *mut MacAddress);
    fn init_bess_eth_ring(ifname: *const u8, core: i32) -> i32;
    fn init_ovs_eth_ring(iface: i32, core: i32) -> i32;
}

pub struct PmdPort {
    connected: bool,
    port: i32,
    rxqs: i32,
    txqs: i32,
    should_close: bool,
    stats_rx: Vec<Arc<AtomicUsize>>,
    stats_tx: Vec<Arc<AtomicUsize>>,
}

impl Drop for PmdPort {
    fn drop(&mut self) {
        if self.should_close {
            unsafe {
                free_pmd_port(self.port);
            }
            self.connected = false;
        }
    }
}

const NUM_RXD: i32 = 256 * 4;
const NUM_TXD: i32 = 256;

impl PmdPort {
    pub fn num_pmd_ports() -> i32 {
        unsafe { num_pmd_ports() }
    }
    #[inline]
    pub fn name(&self) -> i32 {
        self.port
    }

    pub fn stats(&self, queue: i32) -> (usize, usize) {
        let idx = queue as usize;
        (self.stats_rx[idx].load(Ordering::Relaxed),
         self.stats_tx[idx].load(Ordering::Relaxed))
    }

    pub fn new(port: i32,
               rxqs: i32,
               txqs: i32,
               rx_cores: &[i32],
               tx_cores: &[i32],
               nrxd: i32,
               ntxd: i32,
               loopback: bool,
               tso: bool,
               csumoffload: bool)
               -> Result<PmdPort> {
        assert_eq!(rxqs as usize, rx_cores.len());
        assert_eq!(txqs as usize, tx_cores.len());
        let loopbackv = if loopback {
            1
        } else {
            0
        };
        let tsov = if tso {
            1
        } else {
            0
        };
        let csumoffloadv = if csumoffload {
            1
        } else {
            0
        };
        let ret = unsafe {
            init_pmd_port(port,
                          rxqs,
                          txqs,
                          rx_cores.as_ptr(),
                          tx_cores.as_ptr(),
                          nrxd,
                          ntxd,
                          loopbackv,
                          tsov,
                          csumoffloadv)
        };
        if ret == 0 {
            Ok(PmdPort {
                connected: true,
                port: port,
                rxqs: rxqs,
                txqs: txqs,
                should_close: true,
                stats_rx: vec![Arc::new(AtomicUsize::new(0)); rxqs as usize],
                stats_tx: vec![Arc::new(AtomicUsize::new(0)); txqs as usize],
            })
        } else {
            Err(ZCSIError::FailedToInitializePort)
        }
    }

    pub fn new_with_one_queue(port: i32,
                              rxcore: i32,
                              txcore: i32,
                              nrxd: i32,
                              ntxd: i32,
                              loopback: bool,
                              tso: bool,
                              csumoffload: bool)
                              -> Result<PmdPort> {
        let rx_cores = vec![rxcore];
        let tx_cores = vec![txcore];
        PmdPort::new(port,
                     1,
                     1,
                     &rx_cores[..],
                     &tx_cores[..],
                     nrxd,
                     ntxd,
                     loopback,
                     tso,
                     csumoffload)
    }

    pub fn new_loopback_port(port: i32, core: i32) -> Result<PmdPort> {
        PmdPort::new_with_one_queue(port, core, core, NUM_RXD, NUM_TXD, true, false, false)
    }

    pub fn new_simple_port(port: i32, core: i32) -> Result<PmdPort> {
        PmdPort::new_with_one_queue(port, core, core, NUM_RXD, NUM_TXD, false, false, false)
    }

    pub fn new_mq_port(port: i32, rxqs: i32, txqs: i32, rx_cores: &[i32], tx_cores: &[i32]) -> Result<PmdPort> {
        PmdPort::new(port,
                     rxqs,
                     txqs,
                     &rx_cores[..],
                     &tx_cores[..],
                     NUM_RXD,
                     NUM_TXD,
                     false,
                     false,
                     false)
    }

    /// Create a new port that can talk to BESS. Really shouldn't need to do this but fixing this requires patching
    /// DPDK.
    fn new_bess_port(name: &str, core: i32) -> Result<PmdPort> {
        // This call returns the port number
        let port = unsafe {
            init_bess_eth_ring(name.as_ptr(), core)
        };
        if port >= 0 {
            Ok(PmdPort {
                connected: true,
                port: port,
                rxqs: 1,
                txqs: 1,
                should_close: false,
                stats_rx: vec![Arc::new(AtomicUsize::new(0)); 1],
                stats_tx: vec![Arc::new(AtomicUsize::new(0)); 1],
            })
        } else {
            Err(ZCSIError::FailedToInitializePort)
        }
    }

    fn new_ovs_port(name: &str, core: i32) -> Result<PmdPort> {
        match name.parse() {
            Ok(iface) => {
                // This call returns the port number
                let port = unsafe {
                    init_ovs_eth_ring(iface, core)
                };
                if port >= 0 {
                    Ok(PmdPort {
                        connected: true,
                        port: port,
                        rxqs: 1,
                        txqs: 1,
                        should_close: false,
                        stats_rx: vec![Arc::new(AtomicUsize::new(0)); 1],
                        stats_tx: vec![Arc::new(AtomicUsize::new(0)); 1],
                    })
                } else {
                    Err(ZCSIError::FailedToInitializePort)
                }
            }
            _ => Err(ZCSIError::BadVdev),
        }
    }

    pub fn new_vdev(name: &str, core: i32) -> Result<PmdPort> {
        let parts: Vec<_> = name.split(':').collect();
        if parts.len() != 2 {
            Err(ZCSIError::BadVdev)
        } else {
            match parts[0] {
                "bess" => PmdPort::new_bess_port(parts[1], core),
                "ovs"  => PmdPort::new_ovs_port(parts[1], core), 
                 _     => Err(ZCSIError::BadVdev),
            }
        }
    }

    pub fn null_port() -> Result<PmdPort> {
        Ok(PmdPort {
            connected: false,
            port: 0,
            rxqs: 0,
            txqs: 0,
            should_close: false,
            stats_rx: vec![Arc::new(AtomicUsize::new(0)); 0],
            stats_tx: vec![Arc::new(AtomicUsize::new(0)); 0],
        })
    }

    #[inline]
    pub fn copy(&self) -> PmdPort {
        PmdPort {
            connected: self.connected,
            port: self.port,
            rxqs: self.rxqs,
            txqs: self.txqs,
            should_close: false,
            stats_rx: self.stats_rx.clone(),
            stats_tx: self.stats_tx.clone(),
        }
    }

    #[inline]
    pub fn send(&mut self, pkts: *mut *mut MBuf, to_send: i32) -> Result<u32> {
        self.send_queue(0, pkts, to_send)
    }

    #[inline]
    pub fn recv(&self, pkts: *mut *mut MBuf, to_recv: i32) -> Result<u32> {
        self.recv_queue(0, pkts, to_recv)
    }

    #[inline]
    pub fn send_queue(&mut self, queue: i32, pkts: *mut *mut MBuf, to_send: i32) -> Result<u32> {
        if self.txqs < queue {
            Err(ZCSIError::BadQueue)
        } else {
            unsafe {
                let sent = send_pkts(self.port, queue, pkts, to_send);
                let update = self.stats_tx[queue as usize].load(Ordering::Relaxed) + sent as usize;
                self.stats_tx[queue as usize].store(update, Ordering::Relaxed);
                Ok(sent as u32)
            }
        }
    }

    #[inline]
    pub fn recv_queue(&self, queue: i32, pkts: *mut *mut MBuf, to_recv: i32) -> Result<u32> {
        if self.rxqs < queue {
            Err(ZCSIError::BadQueue)
        } else {
            unsafe {
                let recv = recv_pkts(self.port, queue, pkts, to_recv);
                let update = self.stats_rx[queue as usize].load(Ordering::Relaxed) + recv as usize;
                self.stats_rx[queue as usize].store(update, Ordering::Relaxed);
                Ok(recv as u32)
            }
        }
    }

    #[inline]
    pub fn mac_address(&self) -> MacAddress {
        let mut address = MacAddress { addr: [0; 6] };
        unsafe {
            rte_eth_macaddr_get(self.port, &mut address as *mut MacAddress);
            address
        }
    }
}
