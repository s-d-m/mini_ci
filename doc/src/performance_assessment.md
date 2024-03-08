# Performance self assessment


In the end, when I started to measure performance (in a non-scientific way), the numbers I was getting to interact with the webserver
were mostly below 15ms. Unfortunately when looking at the Firefox console I've seen loading time go as high as 100ms (stays mostly on 50-60ms range).
Most of the latency, (Firefox's loading time, network travel time, TCP handshakes etc...) is not something under my control.

At that point, I'm satisfied enough with the performances of the web server per se. I wont pursue the goal of
increased performances any further (even though I have some ideas).

Below are the benchmarks I did. Those were conducted on a commodity hardware, specifically a DELL Latitude
5501 laptop. These benchmarks were executed on a linux virtual machine running under WSL2 in Microsoft Windows
10.

## How long does it take to retrieve the main page?

To check the time it takes to retrieve the main page, the benchmark used was simply to compute the wall clock time
of running `wget` to dowload the main page.

<pre>
<code class="language-shell hljs">shell> hyperfine "wget -q -O - 'http://localhost:3000/'"
Benchmark 1: wget -q -O - 'http://localhost:3000/'
  Time (<span style="color: green;">mean</span> ± <span style="color: #009718;">σ</span>):       <span style="color: green;font-weight: bold;">2.4 ms</span> ±   <span style="color: green;font-weight: bold;">0.5</span> ms    [User: <span style="color: blue;">1.4 ms</span>, System: <span style="color: blue;">0.4 ms</span>]
  Range (<span style="color: #2752d5;">min</span> … <span style="color: #b0147b;">max</span>):     <span style="color: #2752d5;">1.6 ms</span> …   <span style="color: #b0147b;">6.1 ms</span>    <span style="color: #7d7070;">663 runs</span>
</code>
</pre>

It is to be noted that thes timings include the time it takes to start `wget`, create the request, make the
TCP handshake, post the request, receive the response, and report it.  The server's log showed lower times
below one millisecond (reported at 0).

## How long does it take to retrieve the data of a specific build?

The benchmarks are done by simply measuring the time it takes to run a request via `wget`. This is the data for the time it takes
to retrieve the data of a specific job:

<pre>
<code class="language-shell hljs">shell> hyperfine "wget -q -O - 'http://localhost:3000/build/3'"
Benchmark 1: wget -q -O - 'http://localhost:3000/build/3'
  Time (<span style="color: green;">mean</span> ± <span style="color: #009718;">σ</span>):       <span style="color: green;font-weight: bold;">8.4 ms</span> ±   <span style="color: green;font-weight: bold;">0.4</span> ms    [User: <span style="color: blue;">1.5 ms</span>, System: <span style="color: blue;">0.4 ms</span>]
  Range (<span style="color: #2752d5;">min</span> … <span style="color: #b0147b;">max</span>):     <span style="color: #2752d5;">7.5 ms</span> …   <span style="color: #b0147b;">11.2 ms</span>    <span style="color: #7d7070;">273 runs</span>
</code>
</pre>

## How long does it take to retrieve the load the page to enter a build request?

The benchmarks are done by simply measuring the time it takes to run a request via `wget`. This is the data for the time it takes
to retrieve the `add_job` page.

<pre>
<code class="language-shell hljs">shell> hyperfine "wget -q -O - 'http://localhost:3000/add_job'"
Benchmark 1: wget -q -O - 'http://localhost:3000/add_job'
  Time (<span style="color: green;">mean</span> ± <span style="color: #009718;">σ</span>):       <span style="color: green;font-weight: bold;">1.8 ms</span> ±   <span style="color: green;font-weight: bold;">0.4</span> ms    [User: <span style="color: blue;">1.4 ms</span>, System: <span style="color: blue;">0.3 ms</span>]
  Range (<span style="color: #2752d5;">min</span> … <span style="color: #b0147b;">max</span>):     <span style="color: #2752d5;">1.3 ms</span> …   <span style="color: #b0147b;">4.7 ms</span>    <span style="color: #7d7070;">803 runs</span>
</code>
</pre>

## How long does it take to post a job?

The benchmarks are done by simply measuring the time it takes to run a request via `curl`. This is the data for the time it takes
to add a job:

<pre>
<code class="language-shell hljs">shell> hyperfine "curl 'http://localhost:3000/add_job' --compressed -X POST --data-raw '&lt;all the parameters of the add_job method&gt;' "
Benchmark 1: curl 'http://localhost:3000/add_job' --compressed -X POST --data-raw '&lt;all the parameters of the add_job method&gt;'
  Time (<span style="color: green;">mean</span> ± <span style="color: #009718;">σ</span>):       <span style="color: green;font-weight: bold;">15.6 ms</span> ±   <span style="color: green;font-weight: bold;">2.5</span> ms    [User: <span style="color: blue;">3.3 ms</span>, System: <span style="color: blue;">1.3 ms</span>]
  Range (<span style="color: #2752d5;">min</span> … <span style="color: #b0147b;">max</span>):     <span style="color: #2752d5;">12.8 ms</span> …   <span style="color: #b0147b;">28.5 ms</span>    <span style="color: #7d7070;">199 runs</span>
</code>
</pre>

It is to be noted that thes timings include the time it takes to start `curl`, create the request, make the
TCP handshake, post the request, receive the response, and report it.  The timings reported from the server's
log were consistently around 9 to 11 milliseconds with peaks at 13 milliseconds.
