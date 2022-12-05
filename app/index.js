const nostr_tools = require('nostr-tools');
const fetch = require('cross-fetch');
const forge = require('node-forge');
var CryptoJS = require("crypto-js");
// document.addEventListener("DOMContentLoaded", async () => {
//     console.log('dom loaded')
// });
// localStorage.setItem('keys', JSON.stringify({ priv: "myyprivkey", pub: "myypubkey" }));
// localStorage.removeItem('keys');
const pool = nostr_tools.relayPool()
let keys = JSON.parse(localStorage.getItem('keys')) || {}; // {priv, pub };
// let nos2x = window.nostr;
let url = new URL(window.location.href);
let user;
let bodyEl = {};
// let body = document.body

// bodyEl.header = document.createElement("h1");
// bodyEl.signIn = document.createElement("div");
// bodyEl.content = document.createElement("div");
// body.appendChild(bodyEl.header);
// body.appendChild(bodyEl.signIn);
// body.appendChild(bodyEl.content);
bodyEl.header = document.getElementById('header');
bodyEl.username = document.getElementById('username');
bodyEl.menuButton = document.getElementById('menu-button');
bodyEl.menuItems = document.getElementById('menu-items');
bodyEl.logoutButton = document.getElementById('logout-button');
bodyEl.signIn = document.getElementById('signIn');
bodyEl.nos2x = document.getElementById('nos2x');
bodyEl.content = document.getElementById('content');
bodyEl.privKey = document.getElementById('priv-key');
bodyEl.privKeySubmit = document.getElementById('priv-key-submit');
bodyEl.uploadForm = document.getElementById('upload-form');
bodyEl.upload = document.getElementById('upload');
bodyEl.selectedMedia = document.getElementById('selected-media');

async function run() {

    bodyEl.uploadForm.style.display = 'none';
    bodyEl.nos2x.style.display = 'none';
    // let(zero, user, folder) = url.pathname.split('/');
    let headerText = `${url.pathname.split('/').length}`
    if (url.pathname === '/') headerText = `${url.hostname} hostr`
    else if (!url.pathname[3] || url.pathname[3].length === 0) headerText = `${user} homepage`
    else headerText = `${user} ${folder} folder`;

    bodyEl.header.innerText = headerText;
    // bodyEl.signIn.innerText = JSON.stringify(keys);
    // console.log('keys', keys)
    // bodyEl.content.innerText = JSON.stringify(nos2x);

    let menu = createCustomEl('button', 'menu')

    if (keys.pub) loadContent()
    else setTimeout(async () => {
        let pub = await window.nostr.getPublicKey();
        if (pub) bodyEl.nos2x.style.display = 'unset';
        // bodyEl.content.innerText = JSON.stringify(nos2x);
        console.log('keys', keys);
    }, 500);
};

bodyEl.menuButton.onclick = (e) => {
    if (bodyEl.menuItems.style.display === 'none') bodyEl.menuItems.style.display = 'flex'
    else bodyEl.menuItems.style.display = 'none'
}
bodyEl.logoutButton.onclick = (e) => {
    localStorage.removeItem('keys');
    location.reload();
}

bodyEl.privKeySubmit.onclick = (event) => {
    if (bodyEl.privKey.value.length) {
        try {
            let priv = bodyEl.privKey.value;
            let pub = nostr_tools.getPublicKey(priv);
            keys = { priv, pub }
            localStorage.setItem('keys', JSON.stringify(keys));
            loadContent()
        } catch (error) {
            console.log(error);
        }
    }
}
bodyEl.nos2x.onclick = async (event) => {
    keys.pub = await window.nostr.getPublicKey();
    localStorage.setItem('keys', JSON.stringify(keys));
    loadContent()
}
bodyEl.uploadForm.onsubmit = async (event) => {
    event.preventDefault();
    for (let file of bodyEl.upload.files) await saveUpload(file.name, bodyEl.selectedMedia, file)
}
bodyEl.upload.onchange = (event) => {
    // let file = event.target.files[0];
    bodyEl.selectedMedia.innerHTML = "";
    for (let file of event.target.files) {
        hashFile(file);
    }
}
async function loadContent() {
    bodyEl.signIn.style.display = 'none';
    if (keys.priv) pool.setPrivateKey(keys.priv)
    else pool.registerSigningFunction(signAsynchronously)

    let nip05Url = `${url.protocol}//${url.host}/.well-known/nostr.json`;
    let names = JSON.parse(JSON.stringify(await (await fetch(nip05Url)).json())).names
    let users = Object.entries(names).filter(([_, pubkey]) => pubkey === keys.pub);
    user = { name: users[0][0], pubkey: users[0][1] }
    bodyEl.username.innerText = `${user.name}`;
    // fetch(`${url.protocol}//${url.host}/call/${user.name}/search?since=10&tag=meme&tag=test`)
    //     .then(res => res.text())
    //     .then(message => console.log(message))
    // bodyEl.uploadForm.setAttribute('action', `${url.protocol}//${url.host}/call/${user.name}/save?`)
    bodyEl.uploadForm.style.display = 'unset';
    let images = [
        "99bb17dfafc8311c8e9f4935b95dda73fa3cd4ca85b429c4bdba5b56c6216b1c.jpg",
        "37dc8aab92d085ce972cb712b6c2aec92d4cb586bafa0c1208f056f63e0d1731.jpg",
        "304316358f389adb4fb7b527e1ae2f249a149940929e9cff8d8186dc19b02986.jpg",
        "73b60d7f7cad4ca31895d040c7b505131ad0e9e3a4d73c7fec1d47dfa17dcff3.jpg",
        "2a9ef0271b07b32ab13c87e44a98d5ae433fa390ea6e02ad9309eabbae433180.jpg",
        "6bad37fac659f1fb2541d5d5d45ca8f8252e879e3d5e0dee7bf037ebf70962b5.jpg",
        "df5c7e730ae342d42a80191ea46ee05bf7ec9d8d26e166cbb861f06224ae167e.jpg",
        "2fdb76efb058f236459d0211d5ec41251b4bf68ba3e04b9fa6ac8c06d0f9e57c.jpg"
    ];
    for (let image of images) {
        let uri = `${url.protocol}//${url.host}/${user.name}/image/${image}`;
        bodyEl.content.appendChild(createPreview(uri, 'image', image, ['gemma']));
        // bodyEl.selectedMedia.appendChild(createPreview(uri, 'image'));
    }
}
// set sha256 hash of file as file name
function hashFile(file) {
    let reader = new FileReader();
    reader.onload = (e) => {
        let md = forge.md.sha256.create();
        md.update(e.target.result);
        let hash = md.digest().toHex();
        let fileNameSegments = file.name.split('.')
        let ext = fileNameSegments[fileNameSegments.length - 1]
        Object.defineProperty(file, 'name', {
            writable: true,
            value: `${hash}.${ext}`
        })
        bodyEl.selectedMedia.appendChild(createPreview(URL.createObjectURL(file), file.type, file.name));

        // console.log('file hash:', hash, "file", file, "ext", ext, "event.target", e.target.files)
        // let action = bodyEl.uploadForm.getAttribute('action');
        // bodyEl.uploadForm.setAttribute('action', `${action}filename=${hash}.${ext}&`)
    }
    // TODO Remove onload event listener
    reader.readAsBinaryString(file);
}
async function saveUpload(filename, container, file = null) {
    let saveUrl = `${url.protocol}//${url.host}/call/${user.name}/save?`;
    saveUrl += `filename=${filename}&`;
    // let formData = new FormData(bodyEl.uploadForm);
    let formData = new FormData();
    if (file) formData.append('upload', file)
    // for (let entry of formData.entries()) console.log('formData ', entry);
    let tags = [];
    for (let tagEl of container.querySelector(`#${CSS.escape(filename)}`).querySelectorAll('.tag')) tags.push(['t', tagEl.innerText]);
    let content = { method: "POST", uri: saveUrl }
    let nwtEvent = await pool.publish({
        pubkey: keys.pub,
        created_at: Math.round(Date.now() / 1000),
        kind: 1,
        tags,
        content: JSON.stringify(content)
    });
    let nwtEventStringified = CryptoJS.enc.Utf8.parse(JSON.stringify(nwtEvent));
    // let nwtSerialized = nostr_tools.serializeEvent(nwtEvent);
    // let nwtSerialized2 = JSON.stringify([
    //     0,
    //     nwtEvent.pubkey,
    //     nwtEvent.created_at,
    //     nwtEvent.kind,
    //     nwtEvent.tags,
    //     { method: "POST", URI: saveUrl }
    // ])
    // console.log("nwtSerialized", nwtSerialized, "nwtSerialized2", nwtSerialized2);
    // let nwt = Buffer.from(JSON.stringify(nwtEvent)).toString('base64');
    let nwt = base64url(nwtEventStringified);

    let response = await fetch(saveUrl, {
        method: 'POST',
        body: formData,
        headers: {
            "Authorization": `Bearer ${nwt}`
        }
    });
    console.log('saveurl', saveUrl, JSON.stringify(nwtEvent), nwt, 'response', await response.text())
}
// create media preview component
function createPreview(uri, mimeType, filename, tags) {
    let tile = document.createElement('div');
    tile.id = filename;
    tile.style.width = '20rem';
    tile.style.height = '20rem';
    tile.style.position = 'relative';
    let type = mimeType.split('/')[0]
    let media;
    switch (type) {
        case 'image':
            media = document.createElement('img');
            break;
        case 'video':
            media = document.createElement('video');
            media.controls = true;
            media.loop = true;
            break;
        default:
            media = '';
    }
    media.src = uri;
    media.style.objectFit = 'contain';
    media.style.width = '100%';
    media.style.height = '100%';
    tile.appendChild(media);
    let tagList = createCustomEl('div', '', { bottom: '.5rem', right: '.5rem', position: 'absolute', display: 'flex', 'flex-wrap': 'wrap-reverse', gap: '.2rem', 'justify-content': 'end' });
    tile.appendChild(tagList);
    if (tags) {
        for (let tag of tags) addTag(tag, tagList)
        // tagList.querySelectorAll('icon').forEach(icon => icon.style.display = 'none');
        tagList.querySelectorAll('.material-icons').forEach(icon => icon.style.display = 'none');
        let editButton = createCustomEl('button', 'edit');
        let cancelButton = createCustomEl('button', 'close');
        cancelButton.style.display = 'none';
        let saveButton = createCustomEl('button', 'save');
        saveButton.style.display = 'none';
        let deleteButton = createCustomEl('button', 'delete', { top: '.5rem', left: '.5rem', position: 'absolute', cursor: 'pointer' });
        deleteButton.style.display = 'none';
        deleteButton.onclick = (e) => {
            const event = new CustomEvent('edit', {
                bubbles: true
            })
        }
        let tagMenu = createTagMenu({ display: 'flex', background: 'black', 'align-items': 'center' }, tagList);
        tagMenu.style.display = 'none';
        editButton.onclick = (e) => {
            editButton.style.display = 'none';
            cancelButton.style.display = 'unset';
            saveButton.style.display = 'unset';
            deleteButton.style.display = 'unset';
            tagMenu.style.display = 'unset';
            tagList.querySelectorAll('.material-icons').forEach(icon => icon.style.display = 'unset');
            tagMenu.querySelector('input').focus();
        }
        cancelButton.onclick = (e) => {
            editButton.style.display = 'unset';
            cancelButton.style.display = 'none';
            saveButton.style.display = 'none';
            deleteButton.style.display = 'none';
            tagMenu.style.display = 'none';
            tagList.querySelectorAll('.material-icons').forEach(icon => icon.style.display = 'none');
        }
        saveButton.onclick = async (e) => {
            await saveUpload(filename, tile.parentElement);
            editButton.style.display = 'unset';
            cancelButton.style.display = 'none';
            saveButton.style.display = 'none';
            deleteButton.style.display = 'none';
            tagMenu.style.display = 'none';
            tagList.querySelectorAll('.material-icons').forEach(icon => icon.style.display = 'none');
        }
        let controls = createCustomEl('div', '', { display: 'flex', 'flex-wrap': 'wrap', 'max-width': '80%', top: '.5rem', right: '.5rem', position: 'absolute', 'justify-content': 'end' });
        controls.appendChild(editButton);
        controls.appendChild(cancelButton);
        controls.appendChild(saveButton);
        controls.appendChild(tagMenu);

        tile.appendChild(controls);
        tile.appendChild(deleteButton);
    } else {
        tile.appendChild(createTagMenu({ top: '.5rem', right: '.5rem', position: 'absolute', cursor: 'pointer', display: 'flex', background: 'black', 'align-items': 'center' }, tagList));

    }
    return tile
}

function createCustomEl(type, name, style) {
    let el = document.createElement(type);
    if (name.length) el.classList.add('material-icons');
    el.innerText = name;
    for (let prop in style) {
        el.style[prop] = style[prop];
    }
    return el;
}

function createTagMenu(style, tagList) {
    let menu = document.createElement('div');
    for (let prop in style) {
        menu.style[prop] = style[prop];
    }
    // let entry = document.createElement('div');
    let icon = createCustomEl('icon', 'tag');
    let input = document.createElement('input');
    input.type = 'text';
    input.placeholder = 'add tag...';
    input.onkeydown = (e) => {
        if (e.key === "Enter") {
            e.preventDefault();
            button.click();
        }
    };
    let button = createCustomEl('button', 'add');
    // button.type = 'submit';
    // let tagList = document.createElement('ul');
    // tagList.style.width = '100%';
    button.onclick = (e) => {
        let tags = input.value.split(" ");
        tags.forEach(tag => {
            addTag(tag, tagList)
            const event = new CustomEvent('edit', {
                bubbles: true,
                detail: {
                    action: 'add-tag',
                    tag: tag
                }
            })
            menu.dispatchEvent(event);
        });
        input.value = '';
        input.focus();
    };
    menu.appendChild(icon);
    menu.appendChild(input);
    menu.appendChild(button);
    // menu.appendChild(tagList);
    // input.style.display = 'none';
    // input.addEventListener('blur', (e) => {
    //     button.style.display = 'block';
    // })
    // menu.appendChild(button);
    return menu;
}

function addTag(tag, el) {
    if (tag === '') return
    let tagEl = createCustomEl('div', '', {
        background: 'grey',
        borderRadius: '.8rem',
        padding: '.1rem .5rem .3rem .5rem',
        cursor: 'pointer',
        display: 'flex',
        gap: '.3rem',
        alignItems: 'center',
    });
    let icon = createCustomEl('span', 'close')
    icon.style.fontSize = '.9rem';
    let text = document.createElement('div');
    text.innerText = tag;
    text.classList.add('tag');
    tagEl.appendChild(icon);
    tagEl.appendChild(text);
    tagEl.onclick = (e) => {
        tagEl.remove();
        const event = new CustomEvent('edit', {
            bubbles: true,
            detail: {
                action: 'remove-tag',
                tag: tag
            }
        })
        el.dispatchEvent(event);
    };
    el.appendChild(tagEl);
}

// this will try to sign with window.nostr (manual prompt option commented out for now)
export async function signAsynchronously(event) {
    if (window.nostr) {
        let signatureOrEvent = await window.nostr.signEvent(event)
        switch (typeof signatureOrEvent) {
            case 'string':
                return signatureOrEvent
            case 'object':
                return signatureOrEvent.sig
            default:
                throw new Error('Failed to sign with Nostr extension.')
        }
    }
    // } else {
    //   return new Promise((resolve, reject) => {
    //     Dialog.create({
    //       class: 'px-6 py-1 overflow-hidden',
    //       title: 'Sign this event manually',
    //       message: `<pre class="font-mono">${JSON.stringify(
    //         event,
    //         null,
    //         2
    //       )}</pre>`,
    //       html: true,
    //       prompt: {
    //         model: '',
    //         type: 'text',
    //         isValid: val => !!val.toLowerCase().match(/^[a-z0-9]{128}$/),
    //         attrs: {autocomplete: 'off'},
    //         label: 'Paste the signature here (as hex)'
    //       }
    //     })
    //       .onOk(resolve)
    //       .onCancel(() => reject('Canceled.'))
    //       .onDismiss(() => reject('Canceled.'))
    //   })
    // }
}

run();

function base64url(source) {
    // Encode in classical base64
    let encodedSource = CryptoJS.enc.Base64.stringify(source);

    // Remove padding equal characters
    encodedSource = encodedSource.replace(/=+$/, '');

    // Replace characters according to base64url specifications
    encodedSource = encodedSource.replace(/\+/g, '-');
    encodedSource = encodedSource.replace(/\//g, '_');

    return encodedSource;
}

