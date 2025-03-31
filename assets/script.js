


var btn_status = false;
var socket = null;

document.getElementById("clear").addEventListener("click", (e)=>{
    document.getElementById("output").innerText = "";
    document.getElementById("btn").innerText = "OFF";
    btn_status = false;
    if (socket) {
        socket.close();
        socket = null;
    }
})

document.getElementById("btn").addEventListener("click", (e)=>{
    e.preventDefault();
    if (btn_status) {
        e.target.innerText = "OFF"
        socket.close();
        socket = null;
    } else {
        e.target.innerText = "ON"
        socket = new WebSocket('ws://localhost:7143/ws');

        socket.addEventListener('open', function (event) {
            socket.send('Hello Server!');
        });

        socket.addEventListener('message', function (event) {
            console.log('Message from server :', event.data);
            document.getElementById("output").innerHTML += (event.data + "\n");
        });
    }
    btn_status = !btn_status;
})



// setTimeout(() => {
//     const obj = { hello: "world" };
//     const blob = new Blob([JSON.stringify(obj, null, 2)], {
//       type: "application/json",
//     });
//     console.log("Sending blob over websocket");
//     socket.send(blob);
// }, 1000);

// setTimeout(() => {
//     socket.send('About done here...');
//     console.log("Sending close over websocket");
//     socket.close(3000, "Crash and Burn!");
// }, 3000);
