import axios from 'axios';
import { DATA_LOADED, ERROR, LOADGING_IN_PROGRESS, } from './actionTypes'
import { getErrorMessageFromResponse } from '../../../../utils/rest/rest-utils'


export const loadNodeSettings = (host) => {

    return (dispatch) => {
        dispatch({ type: LOADGING_IN_PROGRESS })
        axios({
            method: 'get',
            url: host + "/api/v0/settings",
        })
            .then(function (success) {
                dispatch({
                    type: DATA_LOADED,
                    data: success.data,
                })
            })
            .catch(function (error) {
                dispatch({
                    type: ERROR,
                    errorMessage: getErrorMessageFromResponse(error)
                })
            });
    }
}
