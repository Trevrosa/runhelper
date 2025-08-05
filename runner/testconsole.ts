function wsconsole() {
  let a = new WebSocket("http://localhost:4321/console");

  a.onclose = (_e) => {
    console.log("reconnecting..");
    setTimeout(wsconsole, 1000);
  };

  a.onopen = (_e) => {
    console.log("connected");
  };

  a.onmessage = (msg) => {
    console.log(msg.data);
  };
}

wsconsole();

while (true) {
  const cmd = prompt("> ");
  fetch(`http://localhost:4321/exec/${cmd}`).then(async (resp) => {
    console.debug(await resp.text());
  });
}
