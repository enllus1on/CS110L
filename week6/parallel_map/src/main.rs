use crossbeam_channel;
use std::{thread, time};

fn parallel_map<T, U, F>(mut input_vec: Vec<T>, num_threads: usize, f: F) -> Vec<U>
where
    F: FnOnce(T) -> U + Send + Copy + 'static,
    T: Send + 'static,
    U: Send + 'static + Default,
{
    let mut output_vec: Vec<U> = Vec::with_capacity(input_vec.len());
    for _ in 0..input_vec.len() {
        output_vec.push(U::default());
    }

    // spmc
    let (in_sender, in_receiver) = 
        crossbeam_channel::unbounded::<(usize, T)>();
    // mpsc
    let (out_sender, out_receiver) = 
        crossbeam_channel::unbounded::<(usize, U)>();

    let mut threads = Vec::new();
 
    for _ in 0..num_threads {
        let receiver = in_receiver.clone();
        let sender = out_sender.clone();
        threads.push(thread::spawn(move || {
            while let Ok((idx, num)) = receiver.recv() {
                sender.send((idx, f(num))).expect("send data to output channel error");
            }
        }))
    }

    for _ in 0..input_vec.len() {
        if let Some(num) = input_vec.pop() {
            in_sender.send((input_vec.len(), num)).expect("send data to input channel error");
        }
    }

    // close input/output channel
    drop(in_sender);
    drop(out_sender);

    for thread in threads {
        thread.join().expect("thread panic!");
    }

    while let Ok((idx, num)) = out_receiver.recv() {
        output_vec[idx] = num;
    }

    output_vec
}

fn main() {
    let v = vec![6, 7, 8, 9, 10, 1, 2, 3, 4, 5, 12, 18, 11, 5, 20];
    let start = time::Instant::now();

    let squares = parallel_map(v, 10, |num| {
        println!("{} squared is {}", num, num * num);
        thread::sleep(time::Duration::from_millis(500));
        num * num
    });

    println!("total execute time: {:?}", start.elapsed());
    println!("squares: {:?}", squares);
}
