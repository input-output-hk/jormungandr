import { LOADING_CHANNEL_DATA, CHANNEL_DATA_LOADED, CHANNEL_DATA_LOADING_FAILURE } from './actionTypes'
import { getErrorMessageFromResponse, sendGetMessage } from '../../../../utils/rest/rest-utils'

export const getChannelData = (host) => {

    return (dispatch) => {
        dispatch({ type: LOADING_CHANNEL_DATA });
        sendGetMessage(host + "/api/v0/leaders/logs")
            .then(function (success) {
                dispatch({
                    type: CHANNEL_DATA_LOADED,
                    data: success.data
                })
            })
            .catch(function (error) {
                dispatch({
                    type: CHANNEL_DATA_LOADING_FAILURE,
                    errorMessage: getErrorMessageFromResponse(error)
                })
            });
    }
}