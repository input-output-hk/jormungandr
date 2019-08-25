import { LOADING_DASHBOARD_DATA, DASHBOARD_DATA_LOADED, DASHBOARD_DATA_LOADING_FAILURE } from './actionTypes'

const initialState = {
    isLoading: false,
    error: '',
    data: '',
};

export default (state = initialState, action) => {
    switch (action.type) {
        case LOADING_DASHBOARD_DATA:
            return { ...state, isLoading: true, error: '' };
        case DASHBOARD_DATA_LOADED:
            return { ...state, data: action, isLoading: false, error: '' };
        case DASHBOARD_DATA_LOADING_FAILURE:
            return { ...state, error: action.errorMessage, isLoading: false };
        default:
            return state;
    }
};