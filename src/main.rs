use std::net::{Ipv4Addr, IpAddr, SocketAddrV4, UdpSocket, SocketAddr};
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::{Duration, Instant};
use std::collections::HashMap;

enum Message {
    AddMatchRequest(MatchRequest),
    TriggerTimeouts,
}

struct MatchRequest {
  id: String,
  created_at: Instant,
  source: SocketAddrV4,
}

impl MatchRequest {
  fn build(id: String, source:SocketAddrV4 ) -> Self {
    Self {
      id: id,
      created_at: Instant::now(),
      source: source,
    }
  }

  fn expired(&self) -> bool {
    self.created_at > Instant::now() + Duration::from_secs(120)
  }
}

struct MatchRequestPair {
  mr1: MatchRequest,
  mr2: MatchRequest,
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
          let match_request = MatchRequest::build(id, v4_socket);
          tx1.send(Message::AddMatchRequest(match_request)).expect("tx2 send failed");
        }
      }
    }
  });

  // test udp listener thread
  let tx2 = tx.clone();
  thread::spawn(move || {
    let socket1 = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080);
    let socket2: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 2), 8080);

    let match_requests: Vec<MatchRequest> = vec![
      MatchRequest::build("name1".to_string(), socket1),
      MatchRequest::build("name2".to_string(), socket1),
      MatchRequest::build("name3".to_string(), socket2),
      MatchRequest::build("name1".to_string(), socket2),
    ];

    for match_request in match_requests {
      tx2.send(Message::AddMatchRequest(match_request)).expect("tx2 send failed");
      thread::sleep(Duration::from_millis(2000))
    }
  });
  
  // timeout trigger thread
  let tx3 = tx.clone();
  thread::spawn(move || {
    loop {
      thread::sleep(Duration::from_secs(15));
      tx3.send(Message::TriggerTimeouts).expect("tx3 send failed");
    }
  });
    
  
  // *****************************************
  // Main loop to handle the thread actions
  
  let mut match_requests: HashMap<String, MatchRequest> = HashMap::new();

  for received in rx {
    match received {
      Message::AddMatchRequest(match_request) => {
        println!("Add match request triggered, {:?}", match_request.id);
        match_requests.insert(match_request.id.clone(), match_request);
      },
      Message::TriggerTimeouts => {
        println!("timeout check triggered");
        match_requests.retain(|_, v| !v.expired())
      },
    }
  }


}