function ws() {
  let a = new WebSocket("http://localhost:4321/stats");

  a.onclose = (_e) => {
    console.log("reconnecting..");
    setTimeout(ws, 1000);
  };

  a.onopen = (_e) => {
    console.log("connected");
  };

  a.onmessage = (msg) => {
    console.log(msg.data);
  };
}

ws();
