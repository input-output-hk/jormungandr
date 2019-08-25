import { LOADING_DASHBOARD_DATA, DASHBOARD_DATA_LOADED, DASHBOARD_DATA_LOADING_FAILURE } from './actionTypes'

export const getDashboardData = (data) => {
    if (data === "") {
        data = undefined
    } 
    return (dispatch) => {
        dispatch({ type: LOADING_DASHBOARD_DATA });
        dispatch({
            type: DASHBOARD_DATA_LOADED,
            data: data
        })
    };
}