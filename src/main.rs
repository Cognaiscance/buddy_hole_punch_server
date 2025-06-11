use std::net::{Ipv4Addr, IpAddr, SocketAddrV4, UdpSocket, SocketAddr};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

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
          tx1.send(Message::AddMatchRequest(match_request)).unwrap()
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
      tx2.send(Message::AddMatchRequest(match_request)).unwrap();
      thread::sleep(Duration::from_millis(2000))
    }
  });
  
  

    // thread::spawn(move || {
    //     let vals = vec![
    //         String::from("more"),
    //         String::from("messages"),
    //         String::from("for"),
    //         String::from("you"),
    //     ];

    //     for val in vals {
    //         tx.send(val).unwrap();
    //         thread::sleep(Duration::from_secs(1));
    //     }
    // });

    for received in rx {
      match received {
        Message::AddMatchRequest(match_request) => {
          println!("Add match request triggered");
        },
        Message::TriggerTimeouts => {
          println!("timeout check triggered");
        },
      }
    }


}