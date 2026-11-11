// aria2 JSON-RPC client over HTTP or WebSocket.

import { postJson, openWebSocket, networkError } from './native.js';

const TIMEOUT = 10000;

export class RpcError extends Error {
    constructor(message, code) {
        super(message);
        this.code = code;
    }
}

export class Aria2 {
    #id = 0;
    #pending = new Map();
    #ws = null;
    #wsOpen = null;
    #closed = false;

    /** `onNotification(method, gid)` receives aria2 events when using WebSocket. */
    constructor(url, secret, onNotification = () => {}) {
        this.url = url;
        this.secret = secret;
        this.onNotification = onNotification;
        this.isWebSocket = /^wss?:/.test(url);
    }

    call(method, ...params) {
        return this.#request(this.#payload(method, params)).then((r) => {
            if (r.error) throw new RpcError(r.error.message, r.error.code);
            return r.result;
        });
    }

    /** Runs several calls in one request. Each result is a value or an RpcError. */
    async multicall(calls) {
        const list = calls.map(([method, ...params]) => ({ methodName: 'aria2.' + method, params: this.#params(params) }));
        const results = await this.call('system.multicall', list);
        return results.map((r) => (Array.isArray(r) ? r[0] : new RpcError(r.message, r.code)));
    }

    close() {
        this.#closed = true;
        this.#ws?.close();
        this.#ws = null;
        this.#failAll('closed');
    }

    #params(params) {
        return this.secret ? ['token:' + this.secret, ...params] : params;
    }

    #payload(method, params) {
        const name = method.startsWith('system.') ? method : 'aria2.' + method;
        return { jsonrpc: '2.0', id: String(++this.#id), method: name, params: method === 'system.multicall' ? params : this.#params(params) };
    }

    async #request(payload) {
        if (!this.isWebSocket) {
            const data = await postJson(this.url, payload, TIMEOUT);
            if (!data || typeof data !== 'object') throw new RpcError('Invalid response');
            return data;
        }
        await this.#connect();
        return new Promise((resolve, reject) => {
            const timer = setTimeout(() => {
                this.#pending.delete(payload.id);
                reject(new RpcError('Timeout'));
            }, TIMEOUT);
            this.#pending.set(payload.id, { resolve, reject, timer });
            this.#ws.send(JSON.stringify(payload));
        });
    }

    #connect() {
        if (this.#closed) return Promise.reject(new RpcError('closed'));
        if (this.#wsOpen) return this.#wsOpen;
        this.#wsOpen = new Promise((resolve, reject) => {
            let opened = false;
            this.#ws = openWebSocket(this.url, {
                open: () => {
                    opened = true;
                    resolve();
                },
                close: () => {
                    this.#wsOpen = null;
                    this.#failAll('Disconnected');
                    if (!opened) reject(networkError());
                },
                message: (text) => this.#onMessage(text)
            });
        });
        return this.#wsOpen;
    }

    #onMessage(text) {
        let msg;
        try {
            msg = JSON.parse(text);
        } catch {
            return;
        }
        if (msg.id != null && this.#pending.has(msg.id)) {
            const { resolve, timer } = this.#pending.get(msg.id);
            clearTimeout(timer);
            this.#pending.delete(msg.id);
            resolve(msg);
        } else if (msg.method) {
            this.onNotification(msg.method.replace('aria2.', ''), msg.params?.[0]?.gid);
        }
    }

    #failAll(reason) {
        for (const { reject, timer } of this.#pending.values()) {
            clearTimeout(timer);
            reject(new RpcError(reason));
        }
        this.#pending.clear();
    }
}
