use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::future::Future;
use std::pin::*;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::task::{Context, Poll, Waker};
use std::time::SystemTime;

use rand::RngCore;

struct BoingFuture {
    id: FutId,
    start_time: SystemTime,
    run_time: u32,
}

impl BoingFuture {
    fn new(run_time: u32, id: FutId) -> Self {
        BoingFuture {
            id,
            start_time: SystemTime::now(),
            run_time,
        }
    }
}

impl Future for BoingFuture {
    type Output = FutOut;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let elapsed_ms: u32 = self
            .start_time
            .elapsed()
            .unwrap()
            .as_millis()
            .try_into()
            .unwrap();

        if elapsed_ms >= self.run_time {
            //???
            //let _x = cx.waker().to_owned(;
            //

            Poll::Ready(Box::new(self.id))
        } else {
            //this is a simulation, someone else will call this when it makes sense
            cx.waker().to_owned().wake();
            Poll::Pending
        }
    }
}

struct BoingWaker {
    id: FutId,
    wake_sender: Sender<FutId>,
}

impl std::task::Wake for BoingWaker {
    fn wake(self: std::sync::Arc<Self>) {
        self.wake_sender.send(self.id).unwrap();
    }
}

type FutOut = Box<dyn Send + Sync + std::fmt::Debug>;

type FutId = u32;

//can we lift the box..
type FutureBox = Box<dyn Future<Output = FutOut> + Send + Sync>;

struct Executor {
    wake_receiver: Receiver<FutId>,
    wake_sender: Sender<FutId>,
    tasks: HashMap<FutId, Pin<FutureBox>>,
}

impl Executor {
    fn new() -> Self {
        let (tx, rx): (Sender<FutId>, Receiver<FutId>) = mpsc::channel();
        Executor {
            //should just be scheduler q
            wake_receiver: rx,
            wake_sender: tx,
            tasks: HashMap::new(),
        }
    }

    pub fn spawn(&mut self, future: FutureBox) -> () {
        let id = rand::thread_rng().next_u32();
        self.tasks.insert(id, Box::into_pin(future));
        self.wake_sender.send(id).unwrap();
    }

    //1) making it possible to call the executor from the async funs in a static way, without haveing
    //to know about the executor ref and pass that data aroudn which will be a lot of misery.
    //
    //2) impl spawn, timeout, join, maybe channels
    //
    //shared ownership of the futures, can it be locked in a clever way.
    //how to share work in a smart way
    //who handles what comes from the executor lib etc.

    //multi threaded - task stealing design?
    pub fn start(&mut self, init_future: FutureBox, id: FutId) {
        self.tasks.insert(id, Box::into_pin(init_future));
        self.wake_sender.send(id).unwrap();

        loop {
            let future_id = self.wake_receiver.recv().unwrap();
            println!("going to poll future, {}", future_id);

            let waker = Waker::from(std::sync::Arc::new(BoingWaker {
                id: future_id,
                wake_sender: self.wake_sender.clone(),
            }));
            let mut context = Context::from_waker(&waker);
            let future = self.tasks.get_mut(&future_id).unwrap();

            match Future::poll(Pin::as_mut(future), &mut context) {
                Poll::Ready(val) => println!("finally ready! {:?}", val),
                Poll::Pending => (),
            }
        }
    }
}

async fn boingfuture() -> Box<dyn std::fmt::Debug + Send + Sync> {
    Box::new(39)
}

async fn boingfun(
    e: std::sync::Arc<std::sync::Mutex<Executor>>,
) -> Box<dyn std::fmt::Debug + Send + Sync> {
    println!("wiating for lock");
    e.lock().expect("").spawn(Box::new(boingfuture()));
    println!("done for lock");
    Box::new(1)
}

fn main() {
    //let future_delay_ms = 2000;
    //let (tx, rx): (Sender<FutId>, Receiver<FutId>) = mpsc::channel();

    let id_1 = rand::thread_rng().next_u32();
    let executor = std::sync::Arc::new(std::sync::Mutex::new(Executor::new()));

    //why does rust require send + sync for async
    //the executor needs a channel???

    executor
        .clone()
        .lock()
        .expect("")
        .start(Box::new(boingfun(executor)), id_1);
}
