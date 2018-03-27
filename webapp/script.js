const buying = document.getElementById("buying");
const deleteUserButton = document.getElementById("delete-user-button");
const list = document.getElementById("list-wrapper");
const selling = document.getElementById("selling");
const setUserButton = document.getElementById("set-user");
const settings = document.getElementById("settings-wrapper");
const settingsBox = document.getElementById("settings-box");
const settingsUsername = document.getElementById("settings-username");
const signUpButton = document.getElementById("sign-up-button");
const signUpBuying = document.getElementById("buying-sign-up");
const signUpSelling = document.getElementById("selling-sign-up");
const signUpUsername = document.getElementById("username-sign-up");
const signUpWrapper = document.getElementById("sign-up-wrapper");
const toggleSettingsButton = document.getElementById("toggle-settings");
let intervalID = undefined;

function initialize() {
    let uri = (window.location + "").split("?");
    if (uri.length == 2) {
        let cookie = uri[1];
        let split = cookie.split("&");
        let username = split[0].split("=")[1];
        let token = split[1].split("=")[1];
        confirmUser(username, token);
    } else if (!isNaN(parseInt(localStorage.getItem("token")))) {
        mainLoop(localStorage.getItem("token"),
                 localStorage.getItem("username"));
    }  else {
        initializeSignUp();
    }
}

function mainLoop(token, username) {
    signUpWrapper.style.display = "none";
    list.style.display = "flex";
    settings.style.display = "flex";
    settingsUsername.textContent = username;
    setUserButton.onclick = () => setUser(token, username);
    deleteUserButton.onclick = () => deleteUser(token, username);
    toggleSettingsButton.onclick = toggleSettings;
    getUsers(token, username, list);
    intervalID = window.setInterval(() => getUsers(token, username, list), 30000);
}

function initializeSignUp() {
    list.style.display = "none";
    settings.style.display = "none";
    signUpWrapper.style.display = "flex";
    signUpButton.onclick = signUp;
}

function confirmUser(username, token, callback) {
    let body = {"token": token, "username": username};
    getJson("/api/confirm-user", body, () => {
            localStorage.setItem("token", token);
            localStorage.setItem("username", username);
            window.location = "/index.html";
            initialize();
    });
}

function toggleSettings() {
    if (settingsBox.style.display == "none") {
        settingsBox.style.display = "block";
    } else {
        settingsBox.style.display = "none";
    }
}

function deleteUser(token, username) {
    let body = {"token": token, "username": username};
    if (window.confirm("Are you sure you want to delete your account?")) {
        getJson("/api/delete-user", body, (req) => {
            if (req.response) {
                document.cookie = "";
                localStorage.setItem("username", null);
                localStorage.setItem("token", null);
                location.href = "/index.html";
            }
        });
    }
}

function setUser(token, username) {
    let numBuying = parseInt(buying.value);
    let numSelling = parseInt(selling.value);
    let body = {"token": token, "username": username,
                "buying": numBuying, "selling": numSelling};
    getJson("/api/set-user", body, (req) => {
        if (req.response) {
            // refresh the view
            getUsers(token, username);
        }
    });
}

function signUp() {
    let numBuying = parseInt(signUpBuying.value);
    let numSelling = parseInt(signUpSelling.value);
    let body = {"username": signUpUsername.value,
                "buying": isNaN(numBuying)? 0: numBuying,
                "selling": isNaN(numSelling)? 0: numSelling};
    if (body.username == "") {
        alert("Please enter your husky username to sign up");
        return;
    }
    getJson("/api/sign-up", body, (req) => {
        if (req.response) {
            alert("Success! Please check your email for the sign-in link.");
        } else {
            alert("Oops! There was a problem signing up. Please try again, or "
                  + "send the error message below to coghlan.t for help.\n"
                  + JSON.stringify(req));
        }
    });
}

function getUsers(token, username) {
    let body = {"token": token, "username": username};
    getJson("/api/get-users", body, (req) => {
        // console.log(req.response);
        let rl = list.getElementsByClassName("responsive-list")[0];
        let blackOnWhite = true;
        while (rl.firstChild) {
            rl.removeChild(rl.firstChild);
        }
        for (let user of req.response) {
            if (user.username == username) {
                buying.setAttribute("value", parseInt(user.buying));
                selling.setAttribute("value", parseInt(user.selling));
            }
            let row = document.createElement("div");
            row.className = "responsive-list-item wider";
            if (blackOnWhite) {
                blackOnWhite = false;
                row.className += " black-on-white";
            } else {
                blackOnWhite = true;
                row.className += " white-on-black";
            }
            let link = document.createElement("a");
            link.setAttribute("href", `mailto:${user.username}@husky.neu.edu`)
            link.appendChild(document.createTextNode(user.username));
            link.className = "share-row";
            let buyingSpan = document.createElement("span");
            buyingSpan.appendChild(document.createTextNode(`Buying: ${user.buying}`));
            buyingSpan.className = "share-row";
            let sellingSpan = document.createElement("span");
            sellingSpan.appendChild(document.createTextNode(`Selling: ${user.selling}`));
            sellingSpan.className = "share-row";
            row.appendChild(link);
            row.appendChild(buyingSpan);
            row.appendChild(sellingSpan);
            rl.appendChild(row);
        }
    });
}

function reqIsGood(req) {
    if (req.status != 200) {
        alert("Something went wrong :(. Try signing in again, and contact me"
              + " (coghlan.t) if the problem persists.\n"
              + JSON.stringify(req));
        if (intervalID != undefined) {
            window.clearInterval(intervalID);
            initializeSignUp();
        }
        return false;
    } else {
        return true;
    }
}


function getJson(path, body, callback) {
    let req = new XMLHttpRequest();
    req.open("POST", path);
    req.setRequestHeader("Content-Type", "application/json");
    req.responseType = "json";
    req.send(JSON.stringify(body));
    req.onload = () => {
        if (reqIsGood(req)) {callback(req)}
    };
}

window.addEventListener("load", initialize);
