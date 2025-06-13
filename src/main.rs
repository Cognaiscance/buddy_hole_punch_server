use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket, SocketAddr};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::fmt;

enum Message {
    AddPairRequest(PairRequest),
    TriggerTimeouts,
}

struct PairRequest {
  id: String,
  created_at: Instant,
  source: SocketAddrV4,
}

impl PairRequest {
  fn build(id: String, source:SocketAddrV4 ) -> Self {
    Self {
      id: id,
      created_at: Instant::now(),
      source: source,
    }
  }

  fn expired(&self) -> bool {
    let expiration_time = Duration::from_secs(10);
    let passed_time = Instant::now() - self.created_at;
    let is_expired = passed_time > expiration_time;
    if is_expired {
      println!("Expiring {}", self);
    }

    is_expired
  }
}

impl fmt::Display for PairRequest {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "id: {}, source:{}", self.id, self.source)
  }
}

struct Pair {
  a: PairRequest,
  b: PairRequest,
}

impl Pair {
  fn send_responses(&self) -> bool {
    let socket = UdpSocket::bind("0.0.0.0:6115").expect("failed to connect on port 6114");
    let message_a = format!("id: {}, source: {}", self.b.id, self.b.source.to_string());
    let message_b = format!("id: {}, source: {}", self.a.id, self.a.source.to_string());

    println!("Response to {}: {}", self.a.source, message_a);
    println!("Response to {}: {}", self.b.source, message_b);

    socket.send_to(message_a.as_bytes(), self.a.source).unwrap();
    socket.send_to(message_b.as_bytes(), self.b.source).unwrap();
    true
  }
}


fn main() {
  let (tx, rx) = mpsc::channel();
  
  // udp listener thread
  let tx1 = tx.clone();
  thread::spawn(move || {
    let socket = UdpSocket::bind("0.0.0.0:6114").expect("failed to connect on port 6114");
    loop {
      let mut buf = [0; 1000];
      let (amt, src) = socket.recv_from(&mut buf).unwrap();
      if let SocketAddr::V4(v4_socket) = src {
        if let Ok(id) = String::from_utf8(buf[..amt].to_vec()) {
          let pair_request = PairRequest::build(id, v4_socket);
          tx1.send(Message::AddPairRequest(pair_request)).expect("tx2 send failed");
        }
      }
    }
  });

  // test udp listener thread
  let tx2 = tx.clone();
  thread::spawn(move || {
    let socket1 = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080);
    let socket2: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 2), 8080);

    let request_sequence: Vec<(String, SocketAddrV4)> = vec![
      ("name1".to_string(), socket1),
      ("name2".to_string(), socket1),
      ("name3".to_string(), socket2),
      ("name1".to_string(), socket2),
    ];

    for request_datum in request_sequence {
      let pair_request = PairRequest::build(request_datum.0, request_datum.1);
      tx2.send(Message::AddPairRequest(pair_request)).expect("tx2 send failed");
      thread::sleep(Duration::from_secs(1))
    }
  });
  
  // timeout trigger thread
  let tx3 = tx.clone();
  thread::spawn(move || {
    loop {
      thread::sleep(Duration::from_secs(1));
      tx3.send(Message::TriggerTimeouts).expect("tx3 send failed");
    }
  });
    
  
  // *****************************************
  // Main loop to handle the thread actions
  
  let mut pair_requests: HashMap<String, PairRequest> = HashMap::new();

  for received in rx {
    match received {
      Message::AddPairRequest(incoming) => {
        let search = pair_requests.get(&incoming.id);
        match search {
          None => {
            println!("Adding new PairRequest: {}", incoming);
            pair_requests.insert(incoming.id.clone(), incoming);
          },
          Some(found) => {
            if found.source == incoming.source {
              println!("Updating PairRequest: {}", incoming);
              pair_requests.insert(incoming.id.clone(), incoming);
            } else {
              println!("Requests paired: {}, {}", incoming, found);
              let removed = pair_requests.remove(&incoming.id).unwrap();
              let pair = Pair {a: incoming, b: removed};
              pair.send_responses();
            }
          },
        }
      },
      Message::TriggerTimeouts => {
        pair_requests.retain(|_, v| !v.expired());
      },
    }
  }
}