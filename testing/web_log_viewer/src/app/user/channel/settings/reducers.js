import { DATA_LOADED, ERROR, LOADGING_IN_PROGRESS, } from './actionTypes'

const initialState = {
    isLoading: false,
    isSuccess: false,
    error: '',
    data: '',
};

export default (state = initialState, action) => {
    switch (action.type) {
        case LOADGING_IN_PROGRESS:
            return { ...state, isLoading: true, error: '', isSuccess: false };
        case DATA_LOADED:
            return { ...state, data: action, isLoading: false, error: '', isSuccess: true };
        case ERROR:
            return { ...state, error: action.errorMessage, isLoading: false, isSuccess: false };
        default:
            return state;
    }
};