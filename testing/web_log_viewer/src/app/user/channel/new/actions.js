import axios from 'axios';
import { CREATE_CHANNEL_IN_PROGRESS, CHANNEL_CREATED, CHANNEL_NOT_CREATED, CHANNEL_RESET } from './actionTypes'
import { getErrorMessageFromResponse } from '../../../../utils/rest/rest-utils'

export const reset = (data) => {
    return (dispatch) => {
        dispatch({
            type: CHANNEL_RESET,
            data: data
        })
    }
}

export const createChannel = (host, name) => {

    return (dispatch) => {
        dispatch({ type: CREATE_CHANNEL_IN_PROGRESS })
        axios({
            method: 'get',
            url: host + "/api/v0/node/stats",
        })
            .then(function (success) {
                dispatch({
                    type: CHANNEL_CREATED,
                    data: {
                        response: success.data,
                        host: host,
                        name: name,
                    }
                })
            })
            .catch(function (error) {
                console.log("error")
                console.log(error)
                dispatch({
                    type: CHANNEL_NOT_CREATED,
                    errorMessage: getErrorMessageFromResponse(error)
                })
            });
    }
}
