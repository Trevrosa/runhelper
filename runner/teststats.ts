let a = new WebSocket("http://localhost:4321/stats");

a.onopen = (_e) => {
    console.log("connected")
}

a.onmessage = (msg) => {
    console.log(msg.data.length)
}