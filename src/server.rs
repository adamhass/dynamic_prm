use bytes::{Buf, Bytes, BytesMut};
use hyper::body::Incoming;
use hyper::client::conn::http1::SendRequest;
use hyper::{Method, Request, Response, StatusCode, Uri};
use http_body_util::{BodyExt, Full};
use hyper_util::rt::{TokioExecutor, TokioIo};
use tokio::net::TcpListener;
use plotters::prelude::*;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use std::io::{stdin, Stdin};
use std::sync::Arc;
use std::{env, time::Instant};
use geo::{Contains, Intersects};
use geo::{Line, Point, Rect};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use crate::prelude::*;


pub async fn run_server(addr: String, mut dprm: DPrm) {
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("Listening on http://127.0.0.1:8080");
    loop {
        let con = listener.accept().await.unwrap();
        println!("Received connection, exiting...");
    }
}