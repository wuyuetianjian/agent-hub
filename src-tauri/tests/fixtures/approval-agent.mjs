import readline from "node:readline";
import fs from "node:fs";
const [scenario, receipt] = process.argv.slice(2);
const write = (value) =>
  fs.appendFileSync(receipt, JSON.stringify(value) + "\n");
const send = (value) => process.stdout.write(JSON.stringify(value) + "\n");
readline.createInterface({ input: process.stdin }).on("line", (line) => {
  const message = JSON.parse(line);
  if (message.method === "initialize") send({ id: message.id, result: {} });
  else if (message.method === "thread/start") {
    write({ startup: message.params });
    send({ id: message.id, result: { thread: { id: "test-thread" } } });
  } else if (message.method === "turn/start") {
    send({ id: message.id, result: { turn: { id: "test-turn" } } });
    const method =
      scenario === "question"
        ? "item/tool/requestUserInput"
        : "item/commandExecution/requestApproval";
    const params =
      scenario === "question"
        ? {
            questions: [
              {
                id: "answer",
                question: "TEST ONLY: choose a value",
                options: [{ label: "First" }, { label: "Second" }],
              },
            ],
          }
        : {
            command: "TEST ONLY — no command is executed",
            availableDecisions: ["accept", "decline", { acceptWithExecpolicyAmendment: { execpolicy_amendment: ["test-only"] } }, "cancel"],
          };
    send({
      id: 42,
      method,
      params: { threadId: "test-thread", turnId: "test-turn", ...params },
    });
    if (scenario === "cancel")
      setTimeout(() => {
        send({ method: "serverRequest/resolved", params: { requestId: 42 } });
        write({ cancelled: true });
      }, 100);
  } else if (!message.method && message.id === 42) {
    write({ response: message.result });
    send({ method: "turn/completed", params: {} });
  }
});
