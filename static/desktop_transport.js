(function() {
    'use strict';

    function requestId(prefix) {
        if (window.crypto && typeof window.crypto.randomUUID === 'function') {
            return prefix + window.crypto.randomUUID();
        }
        return prefix + String(Date.now()) + '-' + String(Math.random()).slice(2);
    }

    function isIpcTransport() {
        return typeof window.ipc !== 'undefined' && typeof window.ipc.postMessage === 'function';
    }

    function mapPathToCommand(method, path) {
        const normalizedMethod = String(method || 'get').toLowerCase();
        if (normalizedMethod === 'post' && path === '/api/server/start') return 'epics_server_start';
        if (normalizedMethod === 'post' && path === '/api/server/stop') return 'epics_server_stop';
        if (normalizedMethod === 'get' && path === '/api/server/status') return 'epics_server_status_get';
        if (normalizedMethod === 'post' && path === '/api/modbus/start') return 'modbus_sim_start';
        if (normalizedMethod === 'post' && path === '/api/modbus/stop') return 'modbus_sim_stop';
        if (normalizedMethod === 'get' && path === '/api/modbus/status') return 'modbus_sim_status_get';
        return null;
    }

    function renderStatus(path, result) {
        if (path === '/api/server/status') {
            return '<div id="server-status" class="' + (result.running ? 'success' : 'warning') + ' screen-status-pill"><span>' + (result.running ? 'EPICS Server Running' : 'EPICS Server Stopped') + '</span></div>';
        }
        if (path === '/api/modbus/status') {
            return '<div id="modbus-status" class="' + (result.running ? 'success' : 'warning') + ' screen-status-pill"><span>' + (result.running ? 'Modbus TCP Running' : 'Modbus TCP Stopped') + '</span></div>';
        }
        return '<div class="warning">Unknown status path</div>';
    }

    function renderActionFeedback(path, ok, error) {
        if (!ok) {
            return '<div class="error">' + (error || 'Request failed') + '</div>';
        }
        if (path === '/api/server/start') return '<div class="success">EPICS Server Running</div>';
        if (path === '/api/server/stop') return '<div class="warning">EPICS Server Stopped</div>';
        if (path === '/api/modbus/start') return '<div class="success">Modbus TCP Running</div>';
        if (path === '/api/modbus/stop') return '<div class="warning">Modbus TCP Stopped</div>';
        return '<div class="success">Request completed</div>';
    }

    function deliverStatus(path, response, element) {
        if (!element) {
            return;
        }
        if (!response.ok || !response.result) {
            element.outerHTML = '<div class="error screen-status-pill">Status unavailable</div>';
            return;
        }
        element.outerHTML = renderStatus(path, response.result);
    }

    function deliverActionResult(path, response, feedbackTarget) {
        if (feedbackTarget) {
            feedbackTarget.innerHTML = renderActionFeedback(
                path,
                response.ok,
                response.error && response.error.message
            );
        }

        if (path.indexOf('/api/server/') === 0) {
            const status = document.getElementById('server-status');
            if (status) {
                runStatusRequest(status);
            }
        }
        if (path.indexOf('/api/modbus/') === 0) {
            const status = document.getElementById('modbus-status');
            if (status) {
                runStatusRequest(status);
            }
        }
    }

    function ipcRequest(method, path, element, feedbackTarget) {
        const command = mapPathToCommand(method, path);
        if (!command) {
            if (feedbackTarget) {
                feedbackTarget.innerHTML = '<div class="error">No IPC command mapping for ' + path + '</div>';
            }
            return;
        }

        const request = {
            v: 1,
            kind: 'request',
            id: requestId('transport-'),
            cmd: command,
            token: window.MYCELA_IPC_TOKEN || null,
            payload: {},
            ts: Date.now()
        };

        if (!window.__MYCELA_TRANSPORT_PENDING) {
            window.__MYCELA_TRANSPORT_PENDING = new Map();
        }

        window.__MYCELA_TRANSPORT_PENDING.set(request.id, function(response) {
            if (String(method).toLowerCase() === 'get') {
                deliverStatus(path, response, element);
            } else {
                deliverActionResult(path, response, feedbackTarget);
            }
        });

        window.ipc.postMessage(JSON.stringify(request));
    }

    function httpRequest(method, path, element, feedbackTarget) {
        fetch(path, { method: String(method || 'get').toUpperCase() })
            .then(function(response) { return response.text(); })
            .then(function(html) {
                if (String(method).toLowerCase() === 'get') {
                    if (element) {
                        element.outerHTML = html;
                    }
                } else if (feedbackTarget) {
                    feedbackTarget.innerHTML = html;
                }
            })
            .catch(function(error) {
                if (feedbackTarget) {
                    feedbackTarget.innerHTML = '<div class="error">' + error.message + '</div>';
                }
            });
    }

    function runStatusRequest(element) {
        const path = element.getAttribute('data-myce-status-path');
        const method = element.getAttribute('data-myce-method') || 'get';
        if (!path) {
            return;
        }
        if (isIpcTransport()) {
            ipcRequest(method, path, element, null);
        } else {
            httpRequest(method, path, element, null);
        }
    }

    function bindActionButton(button) {
        button.addEventListener('click', function() {
            const path = button.getAttribute('data-myce-api-path');
            const method = button.getAttribute('data-myce-method') || 'get';
            const feedbackSelector = button.getAttribute('data-myce-target');
            const feedbackTarget = feedbackSelector ? document.querySelector(feedbackSelector) : null;

            if (isIpcTransport()) {
                ipcRequest(method, path, null, feedbackTarget);
            } else {
                httpRequest(method, path, null, feedbackTarget);
            }
        });
    }

    function bootstrap() {
        document.querySelectorAll('[data-myce-api-path]').forEach(bindActionButton);
        document.querySelectorAll('[data-myce-status-path]').forEach(runStatusRequest);

        if (!window.__MYCELA_IPC_DELIVER) {
            window.__MYCELA_IPC_DELIVER = function(response) {
                const pending = window.__MYCELA_TRANSPORT_PENDING && window.__MYCELA_TRANSPORT_PENDING.get(response.id);
                if (pending) {
                    window.__MYCELA_TRANSPORT_PENDING.delete(response.id);
                    pending(response);
                }
            };
            return;
        }

        const previousDeliver = window.__MYCELA_IPC_DELIVER;
        window.__MYCELA_IPC_DELIVER = function(response) {
            const pending = window.__MYCELA_TRANSPORT_PENDING && window.__MYCELA_TRANSPORT_PENDING.get(response.id);
            if (pending) {
                window.__MYCELA_TRANSPORT_PENDING.delete(response.id);
                pending(response);
            }
            previousDeliver(response);
        };
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', bootstrap, { once: true });
    } else {
        bootstrap();
    }
})();