use local_ip_address;


pub fn main() {
    println!("Local IP address: {:?}", local_ip_address::local_ip().unwrap());
}