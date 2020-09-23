
const getJson = async (url) => await (await fetch(url)).json();

export async function list_sessions() {
    await getJson("/sessions")
}

export async function get_session(address) {
    return await getJson(`/sessions/${address}`)
}

export async function send_register(address, sender, signature, sessionKey) {

    const r=  await fetch(new Request(`/sessions/${address}`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify({
            sender: sender.substring(2),
            sign: signature.toString().substring(2),
            sessionKey: sessionKey.encode('hex')
        })
    }))
    return await r.json();
}

export async function send_start(address) {
    return await fetch(new Request(`/admin/sessions/${address}/start`, {method: 'POST'}));
}

export async function send_finish(address) {
    return await fetch(new Request(`/admin/sessions/${address}/finish`, {method: 'POST'}));
}


export async function send_vote(address, sender, vote) {
    const result = await fetch(new Request(`/session/${address}/vote/${sender}`, {
        method: 'PUT',
        headers: {
            'Content-Type': 'application/octet-stream'
        },
        body: new Uint8Array(vote)
    }));

    return await result.arrayBuffer();
}


export async function delete_session(address) {
    return await fetch(new Request(`/sessions/${address}`, {method: "DELETE"}));
}

export async function list_nodes() {
    return await getJson("/nodes")
}

export async function create_session(contract, votingId) {
    const data = {contract, votingId};
    const resp = await fetch(new Request(`/sessions`, {method: 'POST', headers: {
            'Content-Type': 'application/json'
            // 'Content-Type': 'application/x-www-form-urlencoded',
        },body: JSON.stringify(data)}))
    return await resp.json();
}