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

async function commandLoop() {
  while (true) {
    const cmd = prompt(">");
    if (!cmd) {
      // Yield control to allow other async operations
      await new Promise(resolve => setTimeout(resolve, 0));
      continue;
    }
    
    try {
      const resp = await fetch(`http://localhost:4321/exec/${cmd}`);
      console.debug(await resp.text());
    } catch (error) {
      console.error("Error executing command:", error);
    }
    
    // Yield control after each command
    await new Promise(resolve => setTimeout(resolve, 0));
  }
}

wsconsole();
commandLoop();
