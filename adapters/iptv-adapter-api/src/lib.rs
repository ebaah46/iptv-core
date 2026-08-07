#![feature(unboxed_closures)]

pub mod client;
mod dto;
pub mod iptv_rest_client;
mod mapper;

pub use client::HttpClient;
pub use iptv_rest_client::IptvRestClient;
