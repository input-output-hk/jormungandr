import axios from 'axios';

export function getErrorMessageFromResponse(error) {
    let message = 'There is No connection with server'
    if (error.message) {
        message = error.message
    } else if (error.response) {
        message = error.response.data
    }
    return message
}

const config = {
    headers: {
        'Content-Type': 'multipart/form-data',
        'Access-Control-Allow-Origin': '*'
    }
}

export const sendPutMessage = (url) => {
    return axios({
        method: 'put',
        url: url,
        config: config
    })
}
export const sendPostMessage = (url, formData) => {
    return axios({
        method: 'post',
        url: url,
        data: formData,
        config: config
    })
}

export const sendGetMessage = (url) => {
    return axios({
        method: 'get',
        url: url,
        config: config
    })
}
