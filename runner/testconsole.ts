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

process.stdout.write("> ");
for await (const line of console) {
  fetch(`http://localhost:4321/exec/${line.trim()}`).then(async (resp) => {
    console.debug(await resp.text());
  });
  process.stdout.write("> ");
}
