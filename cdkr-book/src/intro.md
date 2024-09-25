# Introduction

CDKTR (con-duck-tor) is a workflow automation and orchestration system written in pure Rust that is designed to take the complexity out of managing and executing tasks across a distributed system. It is designed to be simple to use, and easy to extend.

## Why CDKTR?

The community is awash with different workflow automation and data orchestration tools and platforms, so why was there the need to embark on the (quite frankly) ambitious task to create another one?

CDKTR was designed to solve a problem faced at many businesses which is that a fully centralised workflow automation system is not always the best solution. In many cases, some technical teams only need to automate a few critical jobs, and the overhead of setting up a full-blown workflow automation system is not worth the effort. This is ofen achieved by using a combination of cron jobs, scripts, and other ad-hoc solutions or running open-source tools like Airflow or Prefect.

Another main driver behind CDKTR's development is the nature of many on-prem and cloud environments which are behind corporate firewalls and have strict security policies. This can make it difficult to use cloud platforms that abstract the server and UI away from the user. Another downside is that these services can come with a fairly hefty price tags which leave teams with little choice but to stand up their own instances of open-source tools which then come with a large maintenance overhead.

To this end CDKTR is and will remain completely open-source and free to use. It designed to work efficiently in a variety of environments from a single node setup to a multi-node cluster. 

CDKTR is packaged in a single, small, completely self-contained binary that can be run on any machine even without Rust installed or any other dependencies. This makes it easy to deploy and run in a variety of environments.

## What makes it special?

### TUI
CDKTR is designed to be fairly low-level in terms of implementation, but high-level in terms of the abstractions it provides. This means that it is easy to extend and modify to suit your needs, but also provides a simple interface for users to interact with. This interface is in the form of a Terminal User Interface (TUI) built with [Ratatui](https://ratatui.rs/) which provides a simple way to interact with the system directly from the terminal without having to use or maintain a web interface or other similar tool. This allows teams that usually ssh into their infrastructure to manage and monitor their tasks without having to leave the terminal.

### ZeroMQ

CDKTR uses [ZeroMQ](https://zeromq.org/) (a lightning-fast, no-frills messaging library orginally written in C++) as the messaging layer between the different components of the system. This allows for a high degree of flexibility in terms of how the system is deployed and how the different components communicate with each other. It also allows for a high degree of fault tolerance and scalability without sacrificing performance. It also provides its main API in the form of ZeroMQ sockets which allows for easy integration with other systems developed in completely different languages and frameworks.

### Rust

CDKTR is written in Rust which provides a much higher degree of safety and performance that is often not found in other (most commonly Python-based) workflow automation tools. It allows for managing tasks at rely on low-latency and instantaneous execution times.


## Who is it for?

CDKTR is for developers and technical teams that want a simple, lightweight and scalable way to set up and manage workflow orchestration. I hope you enjoy using it as much as I have enjoyed building it!
