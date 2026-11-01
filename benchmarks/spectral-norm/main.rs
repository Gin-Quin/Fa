//! Spectral norm of an infinite matrix: nested loops over float arrays, integer-to-float conversions.

use std::env;

const DEFAULT_SIZE: i64 = 4_000;

/// The element (i, j) of the matrix A, indexed from 0.
fn element(i: usize, j: usize) -> f64 {
	1.0 / ((i + j) * (i + j + 1) / 2 + i + 1) as f64
}

/// result = A * vector
fn multiply(vector: &[f64], result: &mut [f64]) {
	for (i, slot) in result.iter_mut().enumerate() {
		let mut sum = 0.0;
		for (j, value) in vector.iter().enumerate() {
			sum += element(i, j) * value;
		}
		*slot = sum;
	}
}

/// result = transpose(A) * vector
fn multiply_transposed(vector: &[f64], result: &mut [f64]) {
	for (i, slot) in result.iter_mut().enumerate() {
		let mut sum = 0.0;
		for (j, value) in vector.iter().enumerate() {
			sum += element(j, i) * value;
		}
		*slot = sum;
	}
}

/// result = transpose(A) * A * vector
fn multiply_both(vector: &[f64], result: &mut [f64], temporary: &mut [f64]) {
	multiply(vector, temporary);
	multiply_transposed(temporary, result);
}

/// Reads the size from the first command-line argument, or returns the default.
fn size_argument(default: i64) -> i64 {
	match env::args().nth(1) {
		Some(text) => text.parse().expect("the size must be an integer"),
		None => default,
	}
}

fn main() {
	let n = usize::try_from(size_argument(DEFAULT_SIZE)).expect("the size must not be negative");
	let mut u = vec![1.0; n];
	let mut v = vec![0.0; n];
	let mut temporary = vec![0.0; n];
	for _ in 0..10 {
		multiply_both(&u, &mut v, &mut temporary);
		multiply_both(&v, &mut u, &mut temporary);
	}
	let mut u_dot_v = 0.0;
	let mut v_dot_v = 0.0;
	for (a, b) in u.iter().zip(&v) {
		u_dot_v += a * b;
		v_dot_v += b * b;
	}
	let norm = (u_dot_v / v_dot_v).sqrt();
	println!("spectral norm: {} (x 1e-9)", (norm * 1e9).round() as i64);
}
