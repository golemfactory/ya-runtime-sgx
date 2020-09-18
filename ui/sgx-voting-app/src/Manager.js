
const getJson = async (url) => await (await fetch(url)).json();

export async function list_sessions() {
    await getJson("/sessions")
}

export async function get_session(address) {
    return await getJson(`/sessions/${address}`)
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